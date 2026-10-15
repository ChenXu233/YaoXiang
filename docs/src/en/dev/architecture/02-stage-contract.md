---
frontmatter: {}
---

# Compiler Stage Contracts and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, the grading of acceptance criteria, and the order of execution stages are in the
> RFC-039 main text; the positioning of each subsidiary document is in the
> [index of this directory](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
problem: **"which stages ran in this compilation, which did not, and why" is currently scattered
across six paths and ten entry functions with manual wiring, and no single place can answer this
question.**

The current state can be summarized in one sentence:

> **Invariants that the code itself declares have been broken by architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally states "Unproven → compile
error, no degradation, no silent pass... silent pass must not be resurrected." But this invariant
only holds on the **single-file path**: proof function constraints like `x: Sorted(3)` silently pass
under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP—the proof function is
never executed and no diagnostics are produced.

### Coverage

- The `Stage` enum (exhaustive, no runtime registration) and `StageScope`;
- The `Obligations` ledger and the `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry-function refactoring for all ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and the diagnostic aggregation pattern
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of Scope

- The four-layer model, dependency direction rules, and anti-regression gates → `01-routing.md`
- Equivalence criteria (C1–C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA-ification →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1–P10 global execution order and G1–G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and **in what order** to do it; this document gives the
**concrete form of L1**, the **per-file change list**, and the **implementation stages within this
document** (corresponding to P3 "fix correctness vulnerabilities" and P4 "stage contract and unified
Driver" in RFC-039's global sequence). Where this document conflicts with RFC-039, RFC-039 prevails.

Equivalence criteria are applied under the **C2 (orchestration changes)** category of the
[Equivalence Criteria Document](07-equivalence-oracle.md): identical diagnostic set at each entry +
identical corpus behavior.

## Current State

> This section is entirely **verified facts**, each accompanied by a file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries Are the Only Structural Defect Where "One Wrong Place, Whole Repo Silent"

This is qualitatively different from the other two defect classes (module boundaries, test wiring):

| Defect                | Typical Manifestation          | Has Signal?          |
| --------------------- | ------------------------------ | -------------------- |
| Lexical/syntax error  | Source code written wrong      | Has diagnostics      |
| Type mismatch         | Type written wrong             | Has diagnostics      |
| Missed stage hookup   | A stage is never called        | **No signal at all** |
| Obligation unconsumed | Field filled in but never read | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
is written, not **what is not written**. This is exactly the root cause diagnosed by RFC-039: "This
project treats 'design' as a documentation convention, not as an executable constraint."

### Enforcement Mechanisms Already Present in This Repository

The same repository already has mature enforcement mechanisms; they just haven't been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** in `build.rs:19-55` via `tools/code-tables`, comparing each code line by
  line against the RFC-013 code table, and `panic!`-refusing compilation on any inconsistency.
- `src/package/` (**76 files / 13,012 lines**, with 6,227 lines / 47.9% in the `tests/` subtree) has
  high test density, and each module's test subtree is wired up by declaration—this is the most
  completely wired part of the repo's tests, and can serve as a form reference for the stage-layer
  gate.

**Design capability is sufficient. What's missing is "putting a gate of the same level at the
orchestration layer as well."**

(Line-count metric: `(Get-Content).Count`; see the "Line-count metric" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                              |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ---------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                           |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                               |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + Linking         |
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
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)—RFC-027 Phase 2.5                  |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined within** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The table below gives evidence cell by cell. **An empty cell does not mean "this entry doesn't do
this", but "no code in this entry does this"**—that is the essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                       | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **yes** `187-203`                                   | **no**                                                  | **no**                                                      | **no**                             | **no**                                 |
| 2   | Dead code analysis                                       | yes `275-278` (gated by `config.dead_code.enabled`) | **no**                                                  | yes `356-377` (role-aware, no config gate)                  | **no**                             | **no**                                 |
| 3   | W1006 local module shadowing                             | **no**                                              | **no**                                                  | yes `336-348`                                               | **no**                             | **no**                                 |
| 4   | W1003 unused import                                      | yes (`277` collects `type_result.warnings`)         | **collected but never output**                          | yes `350`                                                   | **no**                             | **no**                                 |
| 5   | W1001/W1002 dead-code family                             | yes (same as 2)                                     | **no**                                                  | yes (same as 2)                                             | **no**                             | **no**                                 |
| 6   | **Monomorphization**                                     | yes `381-389` (gated by `config.mono.enabled`)      | **no**                                                  | N/A                                                         | N/A                                | **no**                                 |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **first-error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, drops `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **no**                                              | **no**                                                  | yes (`321-328`)                                             | **no**                             | **no**                                 |
| 10  | Global slot allocation                                   | N/A                                                 | yes `allocate_global_slots` (`147`)                     | **no**                                                      | **no**                             | **no**                                 |
| 11  | IR generation + qualified name rewriting + linking       | yes (`205`)                                         | yes (`152-236`)                                         | **no**                                                      | **no**                             | yes (independent ModuleIR, then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot at the `9e02e4db` baseline, preserved unchanged. Differences in current state after P3 is
> landed:
>
> - **Row 1 fixed**: proof_execution is shared across the five entries via the same implementation
>   in `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the single-consumer
>   defect no longer exists.
> - **Rows 2/4/5 unchanged in state** (multi-file `run` still does not output W1003, still does not
>   run the dead-code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization is single-file-only) → assigned to WBS 3.4.8 (prerequisite 4.1.3,
>   potential risk not yet proven to be a defect).
> - **Row 7** check_module / check_module_collect_all dual entries → assigned to 4.2.7 (Aggregation
>   parameter-driven).
> - The code-side reference to "ruling #434" was previously unregistered in docs—now back-registered
>   as RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new off-table fact, 2026-10-09 back-registration)**: the
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch); multi-file `discover` previously only recognized
>   explicit `use`—in-project for-loops compile cleanly but at runtime raise E6006 (probe-verified).
>   Fixed and ticked off WBS 4.10.2; the same-family "multi-file missing the single-file step"
>   parse-hard-abort oddity is logged as WBS 4.10.1—**also fixed** (followed immediately after
>   4.2.2; the Check path was downgraded to per-file collection; from the perspective of this audit
>   method's "stage coverage / field consumption" dimension, this is a compilation-unit membership
>   difference, a dimension that slipped through the net).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   The Registry arm re-harvests registrations for embedded std units via `extract_module_info`,
>   which overwrites the native half-face already registered by `with_std()` in "native + yx surface
>   merged" mode (`result.is_err` etc. lost → std.test falsely reports E1043). This audit's third
>   slipped-through dimension: the merge semantics of "two registration sources for the same module
>   key"—the single-file path's registration table is only formed once via `with_std()`;
>   multi-file/Check's Registry arm inserts per-unit, hence the coverage.
>
> **Two places need precise phrasing, otherwise things will be written wrong:**
>
> - **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
>   (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
>   **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report them. The
>   reason is at `compile_project:138`—`type_results.push(result)` stores the full `TypeCheckResult`
>   (including `warnings`), but this function has **no read site for `result.warnings` anywhere in
>   its body**; the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
> - **Row 11's `main` criterion also comes from different sources**: `check_project:385-395` uses
>   `surfaces.bins` (the manifest-declared surface); `compile_project` uses `is_bin_role`
>   (`250-252`, only checking "is there a manifest"). The two functions give different answers to
>   "which files must define `main`".

### Correctness Vulnerability: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. All eight steps below are fully verifiable.**

**Step 1 — The obligation production point is unique.**
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

The semantics: "the predicate argument of this refinement constraint is a compile-time literal, and
the proof function must actually be executed to determine the result."

**Step 2 — The consumption point is inside the checker, but it only emits, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                          | Location                                                                                                     | Behavior                 |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------ |
| Parameter refinement validation | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **emits no diagnostics** |
| Call-site argument validation   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostics** |
| Return-position obligation      | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostics** |

The comment at `checker.rs:5165-5169` literally says:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch is therefore a real line of defense rather than forward-looking placeholder: silent pass must not be resurrected.`

That is, the author explicitly knew that "producing no diagnostics" is a defect, classified it
explicitly as a **known, nowhere-to-go intermediate state**, and the design assumed "someone will
definitely come and read `proof_calls`".

**Step 3 — That field does get populated in the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects into it; `checker.rs:1439` writes it into `TypeCheckResult` with
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is **only read once in production code, at `src/frontend/pipeline.rs:187`** (passing
it as an argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier across the repo fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` are producer-side collection;
`verdict.rs:61` is a same-named field on `ProofResult`; `tests/rfc027_*.rs` read `ProofResult`.)

**Step 5 — All four orchestrator entries bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
in `src/frontend/module/orchestrator.rs` **do not call `pipeline.rs` at all**; they call
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they don't even reach that
single read point at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations of the standard library itself also take the dropping
path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std modules
themselves are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, and produces no diagnostics whatsoever** on
the multi-file / check / LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent drop point inside the layer.**
`checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**`Unproven` from the ownership check layer is swallowed by an empty match arm—no diagnostics, no
bookkeeping.** This directly contradicts the `5164` branch's promise of "silent pass must not be
resurrected", and is **independent of the orchestrator problem**—even if the entry layer is
completely fixed, this one site will still be silent. (Handling timing: should be processed in the
same batch as the obligations ledger, because it is the same class of problem.)

### Why Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three single-file tests for RFC-027
(`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`, `rfc027_refined_transparency.rs`,
`rfc027_return_refinement.rs`) do indeed assert that `proof_calls` is non-empty—for example,
`rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call exists in `result.proof_calls`.
But they **all go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping precisely
before `pipeline.rs`**.

The test file's own doc comment has already stated this. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof call (E4018) is executed by pipeline.rs after check_module, so refinement-violation cases at the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation has been filled in",
while the bug is "the consumer side doesn't read it".** A test that only tests the producer side and
not the consumer side is naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Criteria's Main Corpus Is the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) treats the end-to-end diff over the
293 `.yx` files in `tests/yaoxiang/` as the **third-tier criterion** and the main acceptance method
for category C2 stages. But empirically:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero hits from a directory-wide glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files take the single-file path, all of them cover
  proof_execution, and none of them cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is the
empirical evidence for this. That file is annotated `// expect: compile-error E4018` (line 15), and
its header comment on line 7 says "status: ❌ should be rejected at compile time"—**it passes
precisely because it takes the only path that executes the proof function**.

**Conclusion: the third-tier criterion needs a multi-file corpus layer added, otherwise it cannot
serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 13 "Produced-Equals-Contract" Fields, Zero Mechanism Guarantees Them

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, there are **13 fields** whose
doc comments explicitly state "produced by stage X → consumed by stage Y", meaning they are
essentially **cross-stage obligations**:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-naming convention**: Definition line = the **declaration line** of the field in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of the doc comment—a field's comment block may
> be multi-line, and the declaration line is the only uniquely anchorable position. The **struct
> range** mentioned above (`src/frontend/core/typecheck/types.rs:16-76`) floats with the struct's
> line number and is machine-reconciled by gate **F5** (red on mis-write); the definition-line
> column is reconciled by **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | |
> `release_plan` | `38` | ownership → IR generation (**`PlanId`-keyed**: `ir_gen.rs:360` reads
> (`type_result.release_plan.clone()`), `:3103` uses (`ReleasePlan::consume`); completion marker
> `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | | `instantiation_requests` | `42`
> | typecheck → monomorphization (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck
> → IR generation | | `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` |
> `54` | typecheck → IR generation | | `sum_type_param_names` | `57` | typecheck → IR generation | |
> `operator_dispatches` | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` |
> `62` | typecheck → IR generation (**`PlanId`-keyed**—call-site identity: producer
> `inference/expressions.rs:4550`, consumer `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`);
> completion marker `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) |
> | `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces`
> | `72` | typecheck → IR generation |

**Ruling record (3.4.9, 2026-10-13 user ruling A)**: The original row 14 `interface_impl_registry`
(`types.rs:49`) has been **re-classified as an in-tree fact**—by design it is consumed by typecheck
in-phase, has zero cross-stage reads, and is not an obligation:

- **At in-phase consumption points (active)**: `operator_interfaces.rs:623` (operator queries read
  `env.interface_impl_registry`), seven constraint-solver query sites in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), read-only accessor at
  `checker.rs:214-217`, body_checker injection at `checker.rs:974`.
- **Cross-stage zero reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; on the `src/driver` side only the
  ledger's own registration and documentation (removed in this round).
- **Disposition**: Delete the **declaration** in the doc comment saying "mirrored for middle-side /
  LSP consumption"; preserve the field and its in-phase consumption; the field is registered in
  `NON_OBLIGATION_FIELDS` of `src/driver/obligations.rs` (with reasons). If LSP really needs this
  table in the future, it should be newly registered under 4.6.x; **no consumer is to be fabricated
  in this round**.

**All these fields are passed across layers keyed by `Span` or name table.** The Equivalence
Criteria Document has already pointed out that span-keyed contract mismatches "silently fail with no
error whatsoever" (`ReleasePlan` is typical: any inconsistency in how the two sides compute the span
→ `Drop` instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- `release_plan`'s consumer, `ir_gen.rs:360` (`release_plan: type_result.release_plan.clone()`), is
  inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in the downstream module, and the downstream module is
  on the unavoidable path of every entry.
- `proof_calls`'s consumer is at the **top of `pipeline.rs`**, which is a **different entry
  implementation**. The four orchestrator entries do not go through `pipeline.rs` at all.

**Verified fact**: the repository currently has **no mechanism whatsoever** to ensure that these 13
fields are consumed. The only "protection" is that `ReleasePlan` happened to catch a ride on the
IR-generation path.

### Non-Obligation Field Registry: Channel / In-Phase / In-Tree-Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: fields in `TypeCheckResult` that are not cross-stage
obligations are registered one by one (field name + reason), **complementary** to the "§Obligation
Fields" table above:

| Field                     | Why it is not a cross-stage obligation (key points)                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit identity metadata (0 hits in `src/middle`)                                                                            |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produced-equals-contract")                                                |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but the doc comment does not declare a cross-stage contract ⇒ not collected |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—contract not declared                      |
| `semantic_db`             | Consumer is in LSP (`src/lsp/server.rs:493`), not a compilation-stage obligation                                           |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`, doc comment does not declare a contract                                       |
| `warnings`                | Warning diagnostic channel (same category as `diagnostics`)                                                                |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption points see the "§Obligation Fields" ruling record)                 |

**Why not "wire up a consumer"**: fabricating a downstream for these fields would only introduce
fake obligations, dressing up "by design no cross-stage consumption is needed" as a contract defect;
the correct closure mechanism is **reverse containment**—the field set of `TypeCheckResult` =
obligation list ∪ non-obligation registry (enforced hard by criterion 1 of
`scripts/ci/check-obligations.py`: add a field and forget to register, delete a field and forget to
sync, non-obligation registry missing a reason, or the two tables overlap—red on the spot).

### Four Additional Contract Defects at the Proof Layer

All of the following come from the proof-layer analysis, independent of the entry-fork problem, but
all belong to the category of "stage contracts not being enforced".

**(a) The layer-order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

Line 3 of the README claims "executed in layer order; the upper layer does not run if the lower one
fails". The actual call sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** from the declaration; the
declared Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all**
(it is only used by `inference/assignment.rs:17` as a utility function for `is_subtype`, unrelated
to `ProofResult`). At the same time, **no short-circuit exists**—`checker.rs:1271-1276` `add_error`s
termination errors one by one and continues, so the README's promise of "lower-layer failure
prevents upper-layer from running" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-failure panic has been eliminated by 3.3.1 of P3 (SOLVER
> slot Option-ified, missing treated as conservative degradation to `SMTResult::Unknown`); the
> "silent skip (not injected)" was patched with the W1081 signal by 3.4.2. Unification of the three
> forms (singleton-ification) is performed per **RFC-039 D58**, with the real fix location at
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure port);
> the `default_solver() → &'static` wording in the 02 change list is governed by D58 (the `&'static`
> raw-reference form is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Unavailable                                    |
| ------------------------------------------------------ | -------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Constructed-time injection via `with_solver_owned`       | `None` → **silently skip** (not injected)                  |
| `ownership.rs:627` (back-edge cutoff decision)         | **`default_solver()` called each time**                  | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly opts for hard failure: "initialization failure
remains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds
kernel capability'". But the contract documentation at `backend.rs:60` says "`None`: backend
unavailable... **the caller should degrade conservatively**". **Two philosophies coexist in the same
code, and the hard-failure one panics instead of returning an error.**

**(c) Each back-edge decision creates a new Z3 context; the cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutoff decisions. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every call does `Z3Backend::new()`, i.e., creates a
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". The cache is not shared across
calls ⇒ **this cache will never hit in this call pattern**.

(For comparison, the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifecycle strategies.)

**(d) The checker silently degrades + comments don't match reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, ownership analysis gets an **empty type ledger and an empty call
table** and runs as usual—ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Suggested handling: should be recorded as a warning-level diagnostic, or at least
leave a trace in the obligations ledger.)

The comment at `checker.rs:1244` claims termination check "runs after type checking and before
constraint solving". Actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination is
at `1256`, ownership is at `1296`, i.e., what the comment calls "before" is actually "two layers
after"**.

### wasm Current State: Shim Crate Carries It, the 27-File Branch Is Live

The wasm target **has been built and is built in CI**; `cdylib` is not in the main crate but in an
independent shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has wasm target dependencies           | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` for the corresponding non-wasm segment: tokio/ureq/tempfile)                                                                                                  |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacks into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to a warning if missing)                                                                                                                                                            |
| **27 files** contain the literal `wasm32`         | of which **25** have actual `#[cfg(...)]` attributes, and the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are doc-only mentions                                                                                                                                        |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | all attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate can be called by `wasm/src/lib.rs` (73 lines) under the
wasm target—`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole (all
three require `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path Consuming `proof_calls`               | Whether Proof Function Executes |
| ------------------------------------------ | ------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **yes**                         |
| wasm playground (`wasm/src/lib.rs:48`)     | **yes**                         |
| `build_bytecode` (`lib.rs:171`)            | **yes**                         |
| Multi-file `run` (`compile_project`)       | **no**                          |
| `check` (`check_project`)                  | **no**                          |
| LSP in-project (`check_source_in_project`) | **no**                          |

That is, **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file side. `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" 3), otherwise unifying the Driver will miss this path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario—goes to `06-cleanup-inventory.md`. The RFC-039 open question of "build the wasm
target, or delete these branches" is **already obsolete**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, No Runtime Registration)

**Core constraint: `Stage` is a compile-time exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` enforces at compile time that "a new stage must be
handled by the orchestration layer"—this is exactly the mechanism that the opcode table has already
proven effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check        Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexing + parsing                PerModule
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type checking (with embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis       Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Monomorphization,   // Monomorphization                Project
    Linking,            // Cross-module linking / IR merge Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array
    /// and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at the project level) turn "which stages must run per module and which must run once at the project
level" into a type-level fact. Stages with `Project` scope are structurally guaranteed to run only
once in `dispatch`—this eliminates questions that currently require human reasoning in
`orchestrator.rs`, like "should `allocate_global_slots` be called per file?".

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on the order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the original draft,
> corrected by implementation evidence—
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the original order contradicted the
>    data flow.
> 2. The Check form's stage table is normalized in `Stage::ALL` topological order as **dead code
>    before** proof: `check_project` currently runs proof before dead code, the two have no data
>    dependency, the diagnostic set is the same (C2 set semantics), and only the in-file diagnostic
>    order is normalized. The single-file path (dead code inlined into typecheck, before proof)
>    already conforms to the ALL order, so byte-level acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    original order would force the Registry arm to "hide parse", making the stage table lie about
>    data flow; and multi-file parse thus drops from 2 to 1. The current state is also honestly
>    registered: multi-file path parse errors were once a hard abort (`build_registry_from`'s `?`
>    propagation, the tension with CollectAll semantics is logged as WBS 4.10.1)—**4.10.1 has been
>    fixed** (2026-10-09 user ruling, Rust-style collection semantics + Option B): the Check path's
>    parse failure is downgraded to per-file diagnostic collection, the sick file exits the
>    compilation unit (does not enter registry, the importer reports E5001); the MultiFile path
>    keeps the hard abort (FailFast with a bad file cannot produce IR, which is correct semantics,
>    nailed on the board for the long term).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data-dependency
>    edges 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadow events—the original draft missed these
>    (these products were produced by `discover_with_used` in the orchestrator era and dropped by
>    the `discover` wrapper; the data flow was lost when it wasn't included in the table); b. The
>    in-file diagnostic order normalization is extended to the full form of note #2 above: E3020
>    entry validation is moved into the RoleClassification arm (the Check form has no Linking stage,
>    so the arm holding surfaces and AST is responsible for the validation); the in-file diagnostic
>    order = stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the
>    diagnostic set is unchanged (C2 set semantics); only the order of stderr entries changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "fields are produced but no one consumes them" from undetectable into a fact
that can be asserted at compile time or test time. RFC-039 has already listed this as "the most
valuable single item".

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
    /// Settlement: called once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default)
    /// or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                   | Criterion                                                   | Disposition                                                                           |
| ------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Obligation is read by its declared consumer | The field is `take()`-en / marked consumed in `Obligations` | Pass                                                                                  |
| Obligation non-empty but no consumer        | Field non-empty and not consumed                            | **Produce diagnostic** (`W`-level by default; upgraded to `E`-level in `strict` mode) |
| Obligation empty                            | Field is empty                                              | Pass (no consumption needed)                                                          |

**Why start with W-level, not E-level**: fixing the obligation drop will **change the diagnostic
set**. The C2 criterion requires "the diagnostic set at each entry is the same"; if we go straight
to E-level, `yaoxiang check` will suddenly produce a flood of previously silent `Unproven`
diagnostics. Doing it in two steps (first observe as W, then upgrade to E) keeps every step's
criteria usable. See "Implementation Points" S4 for details.

**`assert_drained()` has exactly one call site**: in `Driver::run`, at the tail of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (a field appearing
≥ 2 times but not read in a third file → fail). `assert_drained()` is the **runtime** gate, that
script is the **static** gate; the two are complementary, and both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: This note records the
> **implemented form, measured numbers, and 4.3.2 interface** of the draft above and the three-tier
> table, mapping item by item to code `file:line`. The landing commit is `1941cc47a` (verification
> tree `99079bc52`). It is also the **third place** in the "no silence" red list (the other two:
> audit surface, static gate).
>
> **I. Landing artifacts (`file:line`)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (`:201`, field name /
>   consumer stage / red-list), **non-obligation field registry `NON_OBLIGATION_FIELDS` (`:150`, 8
>   entries with reasons)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), outbound emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer gate `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the tail of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult` — `src/driver/mod.rs:110` (`State::assert_drained`,
>   `obligations.rs:570`). The early-exit path of `Aggregation::FailFast` settles as well (the books
>   must balance).
> - **Seven delivery points** (one per arm; remove any one and the runtime W1090 must fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is `TypeCheckResult`'s only external
>   read path for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726`
>   (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — four identical delivery
>   ports: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file reads request table directly) / `:1221`
>   (MultiFile·Check: Linking-arm aggregation, `driver/helpers.rs:295` reads per-unit
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation equation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   nailed down by test cases.
> - Diagnostic code: W1090 registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, reason in entries above `:26-31` — settlement granularity is "unit ×
>   obligation", with no user-pointable source position, so no dummy span is stuffed in);
>   six-language copy in `locales/*.json`. **Emission level**: from 4.3.2 onward it is **error**
>   (same code upgraded to E, see part III) — determined by the explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (`obligations.rs:444`), the code number remains
>   in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **II. Cell-by-cell landing of the three-tier table + two extra tiers**
>
> | Situation in the three-tier table above | Implemented criterion                                                                      | Code                     |
> | --------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------ |
> | Obligation read by declared consumer    | **Delivery marker present** (marker ↔ real read reconciled by the static gate, see part V) | `obligations.rs:618-623` |
> | Obligation non-empty but no consumer    | non-empty + consumer stage ran + no marker → W1090 (**error-level**, see part III)         | `obligations.rs:624-630` |
> | Obligation empty                        | `Obligation::is_produced` field-by-field empty check → pass                                | `obligations.rs:587-591` |
>
> Two **added tiers** (2026-10-13 user ruling, not in the draft):
>
> 1. **S1**: the obligation's consumer stage is not in the stage table for this `ProgramKind` →
>    **don't judge as owed** (`obligations.rs:593-597`). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (`program.rs:112-119`), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: the consumer stage is skipped via `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → don't judge as owed (`obligations.rs:598-602`). `NoObligations` does **not**
>    exempt—the obligation is non-empty while this stage declares itself obligation-free, which
>    means the books don't balance, must be reported.
>
> **III. Outbound policy (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: this mechanism only emits **W-level** (W1090); **no strict
> switch**. **4.3.2 (terminal state, 2026-10-13 user ruling A)**: **same code upgraded to E**—the
> `W1090` code number stays, emission level raised to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` `obligations.rs:444`); **no new E code added** (the
> same fact having two codes is parallel representation). **The upgrade only changes the emission
> level, not the emission condition**. **This is a D-class breakage of D59 (changes externally
> observable behavior), honestly logged**: a healthy program emits nothing (when all obligations
> have delivery markers, nothing is reported), so existing corpus and user programs are unaffected
> (342/342 "diff: empty"); but the moment a run actually hits an undelivered obligation, that
> diagnostic **counts into `error_count`, is rendered as `error[W1090]`, and the exit code is
> non-zero**—different from the previous "W-level does not block compilation". The policy that code
> family and emission level are decoupled is logged in the RFC-013 "Emission Channels" section (the
> code-number prefix only determines numbering affiliation, the level is determined by the explicit
> severity at the construction site). Mechanism fact (measured): the severity takes the explicit
> `.severity()`, otherwise "code starts with `W` → Warning" (`codes/builder.rs:186-203`), the
> registry's `ErrorCategory` **does not participate** in severity; the exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) together with `--deny-warnings`
> (`src/main.rs:567-568`), so **W1090 goes into `error_count` because of explicit error (from 4.3.2
> onward)**.
>
> **Red-list fields do not go into the user-visible diagnostic stream** (captain 2026-10-13 ruling
> C; the current red list has been zeroed out, see part IV), and the reasoning is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is fed by a typecheck
>   preset table, **always non-empty**, and what is emitted per compilation is a **constant signal**
>   (no discriminating power), but it would break three existing external contracts—"clean program
>   zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing test cases turning red**, of which **8
>   are on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; another 4 are
>   in `src/driver/tests/`), the corpus baseline changes for 24/342 fixtures, and the only feasible
>   fix is "filter W1090 out of the zero-diagnostic assertion" = loosening an existing criterion.
> - Ruling B (downgrade to Info) still breaks 9/12 of the list-style assertions, and Info is not
>   "emitted at the ruled level".
> - Ruling C measured: existing test cases **zero changes**, corpus baseline **zero changes**
>   (342/342 "diff: empty, behavior equivalent"), and runtime W1090 becomes a **discriminating
>   regression detector**—a healthy tree is silent the whole way, only when a delivery marker is
>   removed does it fire (mutation tests were done by removing markers at three delivery points, CLI
>   `check` prints 2 `warning [W1090]` when the Check-arm marker is removed).
>
> The "no silence" guarantee of the red list is borne by three places: ① the
> `DriverOutcome.obligations.findings` audit surface (carried on every run, including WBS id and
> evidence pointers); ② `check-obligations.py` printing the red list on every run (currently
> empty) + staleness check; ③ this note.
>
> **IV. Red-list current state (cleanup closing) and landing records of 3.4.4 / 3.4.5 / 3.4.9**
>
> Red list 3 → **0**—all three cleanup tasks landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility ("ir_gen
>   generates `CreateVariant`") is equivalently fulfilled by in-place shape detection on the IR
>   side: `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + taking the declaration order from `sum_type_variants`), the call
>   site `:7496` replaces the entire Call with `CreateVariant`; that function's doc explicitly
>   writes "span-keyed cross-system is unreliable (the two sides have different semantics for the
>   span of the Call node), so detect in-place" —**wiring a consumer would bring back the
>   already-fixed span-mismatch defect**. Deletion surface: `types.rs` field, the collection at
>   `checker.rs:1489-1493/:1565`, the collection chain in `inference/expressions.rs` and
>   `inference/statements.rs`, the `VariantCtorCall` struct in `environment.rs`; the type inference
>   and all diagnostics of `infer_variant_ctor_call` **stay**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (kept in the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in-phase
>   (`environment.rs:205-219` registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550` decision point, when there's a unique hit `:4550` writes to
>   `overload_resolutions`); what is deleted is the **zero-reader mirror** at `checker.rs:1558`
>   poured into `TypeCheckResult`. The naming fact still has a single point of expression: the
>   candidate-name table (in-phase) + the span-keyed decision (cross-stage, constructed at
>   `ir_gen.rs:380`, consumed at `:7632/:7705`); the mixed-up names on the definition side are
>   self-computed by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → re-classify as an in-tree fact** (user ruling A, see the
>   ruling record in the §Obligation Fields table of this document). The **declaration** in the doc
>   comment saying "mirrored for middle / LSP consumption" is deleted (`types.rs:47-48`), the field
>   and its in-phase consumption are preserved (`operator_interfaces.rs:623` operator queries, seven
>   constraint-solver queries in `inference/expressions.rs`, the read-only accessor at
>   `checker.rs:214-217`); cross-stage zero reads direct-evidenced by
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire a consumer**: the
>   table's content is the environment mirror filled in by the typecheck preset table (always
>   non-empty); if LSP really needs it, do it via 4.6.x new registration; fabricating a consumer for
>   "a field that by design does not need cross-stage consumption" would only introduce a fake
>   obligation into the ledger.
>
> After the red list is zeroed, **the field census is closed by "obligation list ∪ non-obligation
> field registry (`NON_OBLIGATION_FIELDS`, with reasons)"**— the reverse-containment end of gate
> criterion 1 keeps this tight (adding a field and forgetting to register / deleting a field and
> forgetting to sync both turn red on the spot). The red-list **mechanism** is preserved: gate
> criterion 4's staleness check and the `--selftest` `stale-red-list` fixture continue to cover its
> detection surface, and future re-registration must follow an explicit ruling.
>
> In sync with the cleanup: obligation list 16 → **13** (`INVENTORY` / `Obligation::ALL` / the
> §Obligation Fields table in this document); the gate's `--selftest` now covers seven categories of
> fixtures (newly added "unregistered field", "stale anchor", and "red list is empty" shapes), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Actual criteria of the static gate (9.2)** (three more hard criteria than the one-liner in
> the draft)
>
> 1. **Three-place reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ `TypeCheckResult` field definitions ↔ the "§Obligation Fields" table of
>    this document (including the row count declaration in the table header), red on any drift; and
>    the field set of `TypeCheckResult` must be **exactly equal to** "obligation list ∪
>    non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)"— adding a field and forgetting
>    to register, deleting a field and forgetting to sync, missing reasons in the non-obligation
>    registry, or the two tables overlapping all turn red on the spot;
> 2. **Read-out evidence**: non-red-list obligations must be **really read** in their declared
>    consumer file (`.fieldname` member access; comments / tests / strings don't count)—this turns
>    "delivery marker ↔ real downstream read" into a hard criterion (mechanism A's main risk is
>    exactly the two drifting);
> 3. **Delivery marker**: non-red-list obligations must be marked at the delivery point; conversely,
>    red-list ones must not be marked;
> 4. **Red-list staleness check**: red-list fields must have zero real reads (red the moment a
>    consumer appears), and each entry has a WBS id and evidence pointer; plus `--selftest`: it
>    carries its own negative / positive fixtures; negatives must be red, positives must be green
>    (proves the detector isn't idling).
>
> **VI. Deviations from the draft above, one by one**
>
> 1. The draft's `pub struct Obligations { …fields… }` **is not a field-by-field copy**: the
>    obligation data is still held by `TypeCheckResult`, and the ledger only records "list +
>    delivery markers + settlement conclusion". Reason: a copy would mean two sources of truth (each
>    field is produced respectively by typecheck's checker and the ownership layer), which will
>    inevitably drift; the cross-layer data surface shouldn't be copied into the orchestration layer
>    either.
> 2. The draft's `take()` / "mark consumed" **is not adopted**; it is replaced by `is_produced`
>    empty check + `DeliveryLedger` delivery marker. Reason: the obligation and the product are the
>    same data; `take()` would take away something the downstream needs; and the 11 IrGeneration
>    obligations are in the shape of "hand the whole `&TypeCheckResult` to ir_gen", so per-field
>    `take()` is impossible.
> 3. Draft signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actually
>    `State::assert_drained(&mut self)`: The conclusion is written into `DriverOutcome.obligations`;
>    diagnostics are split by `ProgramKind`: Check/Lsp go through the per-file collection channel;
>    other forms are split by **level** (W1090 is E-level from 4.3.2 onward)—single-file goes to
>    `PipelineError::TypeCheck` (outbound arm at `obligations.rs:654`), multi-file / embedded are
>    otherwise logged as `OrchestratorError` faults (otherwise `result.errors` has no consumer).
>    Reason: returning `Vec<Diagnostic>` would lose shape splitting—check diagnostics must be
>    attributed per file.
> 4. Three-tier-table first-row criterion changes from "obligation read by declared consumer" to
>    "has delivery marker"; "marker ↔ real read" is left to the static gate (part V criterion 2).
> 5. The three-tier-table second-row "`strict` mode upgraded to E" **does not go through a strict
>    switch**; instead it is **upgraded to E in 4.3.2 with the same code** (upgraded even without a
>    strict switch, because the healthy state emits nothing, which is already safe enough, R4).
> 6. The static-gate criterion goes from "field appears ≥ 2 times but no third-file read" to the
>    four criteria in part V.
> 7. The two added tiers (S1, skip exemption) not in the draft are in part II.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table: actually landed
>    as **20 test cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turned
>    green; red state archived: after removing the sole settlement point, `pairs: 0`, 6 failures).
>    The S1/S4 two lines have been synced to the actual file.
>
> **VII. Landing record of 4.3.2 (level finalization + gate hardening)**
>
> - **Per-field review input**: `ObligationSettlement.drained_kinds` (the kinds of obligations
>   actually drained in this run, obtainable per run) and the five counts (the conservation equation
>   in part II); combined with the audit surface `findings`, it can give the fact "whether this
>   field has been consumed by any entry".
> - **Level: same-code-upgraded-to-E has landed** (2026-10-13 user ruling A; see the terminal-state
>   registration in part III and the D59 D-class breakage declaration). Implementation: explicit
>   `.severity(Severity::Error)` in `to_diagnostic`; the outbound arm splits the error channel by
>   level. Mutation evidence: remove any delivery marker → that form's compilation fails, `check`
>   exit code is non-zero (`error [W1090]`); after restoration, the healthy state emits nothing
>   (342/342 corpus, zero changes). The locale copy in all six languages already states "W-family
>   code, emission level is error".
> - **Gate hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses **`file:line` anchors** in
>     non-obligation-registry reasons and red-list evidence, validating "file is locatable
>     (unique) + line number is in range + the symbol immediately following the anchor actually
>     appears in the anchored file"; red the moment the anchor points to a non-existent symbol. It
>     carries its own negative fixture `tests/baselines/obligations-gate/stale-anchor/` (one each
>     for non-existent symbol and out-of-range line). Note: the red list is currently empty, so the
>     actual landing point of this criterion is the **non-obligation registry** and future evidence
>     pointers (same code path).
>   - **F2/R3 reverse containment**: the gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registry count = difference", red if the difference is non-empty;
>     carries its own negative fixture `unregistered-field/` (struct adds a new field without
>     registration).
> - **Information item F3**: The `SkipReason::UpstreamFailed` exemption is unreachable on real
>   entries (units that fail typecheck don't keep their `type_result`, so the obligation doesn't
>   exist in the first place); this tier is pinned down by synthetic `State` test cases, and the
>   real end-to-end only covers `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 second-level task closing (red list zeroed + obligation diagnostic terminal state
> E-level)**
>
> **8.1 Three-round landing timeline** (each item maps to code `file:line`; this round only adds the
> conclusion, doesn't change code)
>
> | Round               | Conclusion                                                                                                                                                                                   | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
> | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger online | `State::assert_drained()` sole settlement point; 13-entry obligation list + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                               | call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; outbound `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                                 |
> | 3.4.4 cleanup       | `variant_ctor_calls` delete the field—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                       | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                             |
> | 3.4.5 cleanup       | `method_overload_ir_names` delete the cross-stage mirror (kept in the in-phase candidate-name table + `overload_resolutions`)                                                                | decision consumption point `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                          |
> | 3.4.9 cleanup       | `interface_impl_registry` re-classify as **in-tree fact**: delete the "mirrored for middle / LSP consumption" declaration, preserve in-phase consumption; obligation 14 → 13, red list 1 → 0 | declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten by ruling A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven sites in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor `:214-217`); registration `src/driver/obligations.rs:150` |
> | 4.3.2 level         | **Same code upgraded to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; outbound splits the error channel by level (first-failure precedence)                         | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); the registration of code-family/level decoupling is in the "Emission Channels" section of `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                  |
>
> **8.2 Closing conclusion**
>
> - **Red list zeroed**: the number of entries in `INVENTORY` (`src/driver/obligations.rs:201`) with
>   `red_list: Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The
>   mechanism **is preserved**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion
>   4's staleness check, and the `stale-red-list` fixture; the gate outputs "red list 0
>   entries—cleaned up".
> - **Obligation diagnostic terminal state E-level**: the **code number and W-family affiliation of
>   `W1090` are unchanged**, the emission level is `error` (D5 terminal state, 2026-10-13 user
>   ruling A). This **is a D-class breakage of D59** (changes externally observable behavior), and
>   has been double-registered in part III of this note and the "Emission Channels" section of
>   RFC-013; **the healthy state emits nothing ⇒ existing corpus and user programs are unaffected**
>   (342/342 corpus "diff: empty", IR snapshot zero drift; t11 independent verification and t12
>   re-review each re-checked).
> - **Field census closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation registry
>   (8 entries, `src/driver/obligations.rs:150`, each with a measured consumption-side reason); gate
>   F2 explicitly outputs the difference (currently `21 − 13 − 8 = 0`).
> - **No un-landed promises**: all the hardening items registered in this note have been **landed**
>   (F1/F2/F4/F5, see parts V, VII, VIII); the only two items not closed are the **known limits** in
>   8.4 and t7 F5, which has been moved to an issue, neither of which is an un-landed criterion of
>   this mechanism.
>
> **8.3 Disposition of findings from each round's acceptance / re-review** (one by one)
>
> | Source        | Finding                                                                                                                                                                    | Status                                                                                                                                                                                                                                                                                            |
> | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is plain-text pointer, silently stale when the substitute consumer is removed                                                                            | **Landed**: gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                                                  |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, silently PASSes when a new field is forgotten                                                                                  | **Landed**: F2 explicit difference output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                                                 |
> | t4 R4         | The W-only vs upgrade-to-E policy is unregistered                                                                                                                          | **Landed**: parts III / VII of this note + RFC-013 "Emission Channels" + W1090 help and `error_output` samples in all six `locales/*.json`                                                                                                                                                        |
> | t7 F5         | Dead-code analysis miscounts overloaded method references (legal overloads still get false `W1005`)                                                                        | **Moved to issue `#450`**, attributed to WBS **4.9 reference pool correct implementation** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`)—not part of this second-level task                                                                                                 |
> | t11 F7        | The gate's `--root` mode does not cover anchor promises in module-header prose comments                                                                                    | **Known limit** (see 8.4)                                                                                                                                                                                                                                                                         |
> | t11 F8        | Validator's own tooling self-record: mutation driver once wrote to the main repo by mistake (cleared by immediate `git restore`, affected rounds' output voided and rerun) | **Information item** (not a deliverable defect)                                                                                                                                                                                                                                                   |
> | t11 F9        | Intermediate state of main repo (un-staged files make this document's table header out of sync with `INVENTORY`) triggers gate red                                         | **Information item / resolved**: frozen-commit self-consistent PASS; "committing in intermediate state directly turns the gate red" is exactly the design intent of F2/F4                                                                                                                         |
> | t12 F12-1     | This document's struct range is written as `16-73`, but actually `16-76` (no machine reconciliation after t9 added 3 lines of comments)                                    | **Landed + mechanized**: range corrected to `16-76`, and new gate **F5** added (range anchor, red on mis-write); the same family of numbers is grep-aligned in one pass (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                                       |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was covered by W1090)                                                                  | **Fixed** (captain has amended the commit message to explain): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned-board** test case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a newly added test |
>
> **8.4 Known limits (registered items, not un-landed promises)**
>
> - **F1's scan surface**: anchor validation only covers the text of **registered entries**—the
>   non-obligation-registry reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence; **`file:line`
>   promises in module-header prose comments are not in the scan surface** (currently no registered
>   entries are affected). If in the future an anchor promise is written in a module header, it must
>   be **borne by human review**, or F1's scan surface expanded—choose one, do not default to no one
>   in charge.
> - **Static limits of the gate**: the set-level check of delivery markers is on the static side
>   (criterion 3), and single-arm removal is filled by **runtime settlement**— static + runtime
>   double-layer is a designed shape, not a gap (t11's M11A / M11B / M11C mutation bidirectional
>   verification).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization
> landing)**: Of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) per RFC-039
> **D20**. This note records the landed form, criteria, classification basis, and known limits; the
> definition lines are still reconciled by gate **F4** (the two rows in the table above are the
> current code facts).
>
> **I. Landed form (carrier option A: `PlanId`)**
>
> The key is a **structural identity** that follows the AST node, no longer relying on the
> positional coincidence of "each side computes a span": `PlanId` (`src/util/plan_id.rs`,
> monotonically increasing `u32`, unique within a compilation unit, `UNPLANNED = u32::MAX` for
> synthetic nodes outside the parser) is issued by the **sole allocator**
> `ParserState::alloc_plan_id`. Two carriers: `Stmt.plan_id` (statement identity → release plan's
> ownership and consumption) and `Expr::Call.plan_id` (call-site identity → overload resolution's
> production and consumption); `Expr::call` is the only call-expression constructor in the repo, and
> synthetic calls outside the parser are uniformly marked `UNPLANNED` (**cannot be consumed**, see
> part II).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the only
>   consumption entry point is `ReleasePlan::consume(stmt)`—**reading and bookkeeping happen in the
>   same call**; `ir_gen.rs:360` reads, `:3103` uses (the statement identity is given by
>   `walk_stmt`'s push/pop + CFG-node→statement-identity table; the Drop ownership of a scope exit
>   belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **co-located**), the
>   production point is `inference/expressions.rs:4550` (written by call-site `PlanId`, dispatch
>   goes through the destructuring of `Expr::Call { plan_id }` for transparent pass-through),
>   consumption points `ir_gen.rs:7697` (with binding) / `:7772` (no binding) via
>   `OverloadResolutions::get` (hit-then-bookkeep).
>
> **II. Consumption-must-reach settlement (reusing the 4.3 ledger, no second set)**: The consumption
> ledger is the **single implementation** `ConsumptionLedger<K>` (`src/util/consumption.rs`), each
> of the two tables holds one instance; when `generate_module_ir` returns successfully, the same
> point marks both ledgers' "completed a full round" (`ir_gen.rs:1095` for release plan / `:1097`
> for overload resolution); the tri-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), and the sole settlement point adds one row per obligation—the
> difference being non-empty is **E-level W1090** (same code upgraded to E, see part III). The
> exemption criterion hangs on the **ledger's own completion marker** (not whether `unit.ir`
> exists), so the Check form taking the IR product does not constitute an exemption.
>
> **III. D59 classification basis (C-class, red-then-green)**: The shape
> `Type.method(positional, name = value)` was previously **silently giving the wrong answer**
> (measured: `r: Int = t.pick(1, a = 2)` compiles cleanly, runs with output `1` instead of `20`—the
> resolved candidate is dropped, falling back to the last binding in declaration order); after the
> change, the same program is rejected at compile time with
> `error [W1090] ... production key unconsumed`. The fix takes the **loud** rather than "stuff the
> resolution name into a named-argument call": the latter would lose the named arguments, equivalent
> to a different kind of silence (terminal semantics in [09](./09-execution-wbs.md) §3.14 / issue
> #452). Also: `Drop` is a no-op in the interpreter
> (`src/backends/interpreter/executor/ops/control.rs`'s wildcard arm only `advance()`), so the key
> migration of the release plan doesn't change runtime behavior—342 corpus diffs empty is the
> empirical evidence.
>
> **IV. Criteria and evidence**: D41 same-type criterion `test_release_plan_spans_consumed` (settled
> by `PlanId`) and newly added `test_overload_resolutions_consumed` (per-file corpus
> lex→parse→typecheck→ir_gen, **difference must be empty + evidence surface floor**: number of files
> that produced a resolution `≥ 1`, real consumed entries `≥ 2`, no whitelist); 9-item
> decision-shape coverage pinned at `src/frontend/core/typecheck/tests/overload_resolutions.rs`
> (expected return / positional argument / type-body binding / `?` operand / pure named /
> positional+named / ambiguity / variant construction / unallocated identity). Mutation driver:
> consumer key shifted by one ⇒ criterion red + real program `error [W1090]` + corpus gate diff;
> green again after restoration. Gate numbers: `check-obligations.py` PASS, 342 corpus diffs empty,
> `cargo test --lib` 2712 passed / 0 failed (IR snapshot zero drift).
>
> **V. Known limits (registered items, not un-landed promises)**: The **no-binding consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable** on the current corpus and test surface (method
> declarations uniformly go through the with-binding consumption point via
> `register_type_bindings`), so this branch's key-control correctness **has no red state to
> falsify**; the reason for keeping it = it is the symmetric branch of the with-binding path;
> removing it would rewrite "no `type_bindings` method call" into silence. Recorded by independent
> verification (t5 F-1) and re-review (t6 F2), **no new task opened**.

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
    /// Can only come from one of the 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be the above 6 predefined combinations (one per `ProgramKind`), and does not
accept arbitrary arrays from the caller.** This is the key constraint that prevents the `Program`
abstraction from degrading into "a free parameter that accepts anything", enforced by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides whether to skip, not the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one less
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the draft above, semantically equivalent, registered for future reference—
>
> 1. The `StageOutcome` tri-state in §4 is not returned by the stage arm, but expressed by `State`
>    failure marker + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, eliminating
>    per-arm boilerplate returns);
> 2. `Driver` has no `config` field—configuration has a single source in `Program.config` (the §5
>    "avoid Driver holding mutable global state" ruling covers the draft field);
> 3. The `Skipped` diagnostic is only recorded into the internal `DriverOutcome.skipped` ledger in
>    4.1.3, not emitted—the zero-diff criterion of S2 "deliberately fix no bug" requires it;
>    emission comes with the 4.3 obligation ledger;
> 4. The `proof_execution` module's visibility is relaxed to `pub(crate)`: the driver arm is the
>    only new caller (L1→L2 is an allowed direction); the orchestrator's existing call sites will be
>    revisited after migrating away in 4.2.
> 5. Under the multi-file form, ProofExecution is an independent stage, executed after **all**
>    typechecks (A1, 2026-10-09 user ruling): the original `compile_project` interleaved proof
>    within the per-file typecheck loop (file N's proof before file N+1's typecheck). Single-failure
>    both are byte-identical; in the multi-failure scenario of "file1 proof failure + file2
>    typecheck failure", the first report changes from proof error to typecheck error (test pinned).
> 6. `DriverOutcome` carries the product by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface that the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have empty Vec—the current contract of
>    `check_project`); Discovery's product is extended to a "file set + used_by edge set + shadow
>    events" triple entering State (the MultiFile form has no downstream consumer, recorded only and
>    not emitted); the Check path's per-file parse drops from 2 to 1 with the C3 topology.
> 8. Lsp form landing (4.2.3, split by file source ruling, 2026-10-09 user decision): Disk-file
>    parse failure is the same as Check's Option B (collect + exit compilation unit—two-side
>    degradation converges); the edited buffer's parse failure keeps the partial AST and continues
>    to typecheck (editor philosophy—a mid-typing state shouldn't make semantic features disappear).
>    The Discovery arm keeps the buffer source for Lsp overriding stale disk content; Typecheck
>    under Lsp only checks the target file (other units only feed the registry for signatures); the
>    in-file diagnostic order normalization principle extends to LSP (typecheck diagnostics → W-code
>    warnings → proof errors). The old behavior where LSP hard-aborted on irrelevant disk file parse
>    errors and the handler silently fell back to the single-file path is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` adds a `shared_registry` field—the embedded std module
>    is a sub-compilation, the registry is an **input** rather than a product (EMBEDDED_ STAGES has
>    no Registry stage), the SymbolTable sharing contract of #94 is thereby made explicit as a
>    program declaration from "the caller remembers to pass the same registry"; error-path text
>    normalized from `<std.test> (embedded std)` to the unit virtual path `<std/test>` (only the
>    compiler's internal error surface, no tests pinned). The orchestrator's four entries have now
>    all migrated to Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): The Check form's
>     faithful carrying of single-file semantics for programs without a project root—warning surface
>     only covers the entry file (neighboring files only get errors, better to miss than to be
>     wrong); relative `use` resolves along the importer's directory (rustc single-file mod
>     aligned). CHECK stage table adds GlobalSlotAlloc + IrGeneration: the standalone old path
>     (pipeline full chain) already runs IR generation, E3019/E1014/E1015 etc. are only produced in
>     ir_gen (runner gate empirical evidence). IrGeneration's Check form is pure checking without
>     consuming IR; the Script/Bin division is expressed by module_key's None/Some (the existing
>     switch of E3023, `ir_gen.rs:1660`). Monomorphization does not enter CHECK (only produces
>     resource overruns / internal errors, zero corpus dependency). **〔2026-10-10 withdrawn by
>     3.4.8 empirical evidence〕**—"zero corpus dependency" is actually evidence of a blind spot
>     where the corpus lacks pathological recursive fixtures; E3005 is the unique compile-time
>     defense for this class of programs. See note #14. Incidental fix for a latent defect: the
>     Registry arm's repeated harvesting of registration for embedded std units overwrites the
>     native half-face already merged by `with_std()` (E1043 false positive)—the embedded unit now
>     skips duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted, and goes through
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, keeps the partial AST and
>     continues to typecheck (editor philosophy, extended from the 4.2.3 ruling; not marking the
>     stage as failed, otherwise topology skipping makes typecheck unreachable); Typecheck
>     dispatches the `check_module_collect_all` free function. The LSP single-file path thus catches
>     up on proof (E4018), W-code warnings (W1001–W1005, test pinned) and IR-level errors; lex
>     failures now report real diagnostics (the old synthetic "E0001 lex error" text vanishes with
>     the sequence's deletion—real diagnostics carry precise spans, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 required
>     item): the two entries have long since converged internally—the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single-bool, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain modes both run, the difference carried by whether
>     `collected_errors` is empty; five collection points in statements.rs (function body / use /
>     for / block / while). During verification, the duplicate-diagnostics defect was confirmed and
>     fixed: the collection point put the first error into both `collected_errors` and the `Err`
>     return channel, and pass-3's `add_error` on `Err` caused same-code-same-span-same-message ×2
>     (nested up to ×3); there are also two independent mechanisms: annotation validation's
>     signature-formal-and-whole-annotation double-visit (E1003/E1103), and the ownership layer's
>     same-point double-emit (E2014/E2018). The fix is at the module result boundary: deduplicate by
>     (code, span, message)—diagnostics are positional facts, duplicates carry no information; 63
>     corpus entries have duplicate copies removed, the run column and exit code have zero drift
>     (`borrow_conflict_err`'s "duplicate" is an artifact of the baseline triple not containing a
>     column number—two E2018's are actually legitimate diagnostics at different columns).
> 13. wasm playground landing (4.2.8): `run_code` / `test_compile` go through `compile_playground`
>     to construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper layer is no
>     longer in the way. Error text byte-aligned with the old `CompileError` Display prefix (Parse
>     error: / Type error: / Internal error:), playground UI zero visible difference. WasmPlayground
>     and SingleFile share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step gives
>     it a real producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Empirically overturns "mono only
>     produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) on the no-mono
>     multi-file path causes the compiler process to stack-overflow (0xc00000fd—the interpreter
>     recurses at the Rust layer, with no graceful runtime error), the mono depth gate is the only
>     compile-time defense; standalone check before 4.2.5 went through the pipeline full chain and
>     already had mono, the exclusion ruling caused a silent coverage regression ("zero corpus
>     dependency" only proves the corpus has no pathological fixtures). The "type erasure fallback"
>     assumption is voided by the RFC-033 reflection ruling—`^^List(Int)` needs the real identity of
>     instantiation, monomorphization is the foundation of reflection, not an optional optimization.
>     Four design points: ① typecheck produces instantiation_request for qualified calls
>     (`lib.f(x)`, FieldAccess shape), generic_id uses the qualified name, in the same namespace as
>     merged IR (currently only recognizes bare Var, `expressions.rs:2455`); ② `Stage::ALL` topology
>     is changed to Linking before Monomorphization, MULTI_FILE stage table tail adds
>     Monomorphization— consumes merged_ir, aggregates all-unit requests (the deferred bucket's
>     `containing_fn` qualified), mono is elevated from a single-module pass to a whole-program
>     pass; single-file stage table stays unchanged (no Linking, subsequence property preserved, IR
>     snapshot zero drift); ③ CHECK adds Linking (pure merge—E3020 entry validation stays in
>     RoleClassification, to prevent double reporting) and Monomorphization (pure check without
>     consumption, same as IrGeneration's Check form) two stages; ④ resource protection follows the
>     whole-program BFS to cover cross-unit mutual recursion. The Monomorphizer itself doesn't
>     change (consuming ModuleIR + request set is exactly the merged IR shape).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per the #14 design, qualified-call instantiation
>     collection is landed: `ExpressionInferrer` is injected with the module-qualified key table
>     (registered through three arms of `use`—normal / alias / per-item alias and only SubModule
>     kind exports), `callee_generic_name` is concatenated via `SymbolTable::qualify` to give a
>     qualified name from the same source as merged IR; qualified call's first-path arity criterion
>     is tightened (when the parameter table can't be resolved, fall back to the second path and
>     take actual arguments from the signature, to avoid false E3018 from cross-module same-name
>     function mismatch). During implementation, three **asymmetry defects on the mono side** were
>     empirically confirmed and fixed in the same round: (1) The delete key (generic name set) and
>     rewrite key (`containing_fn` + name + span triple) granularity mismatch—when the actual
>     arguments can't be resolved, the site degrades to a symbol request entering the deferred
>     bucket, when the container function is non-generic the bucket never drains, yet the original
>     is already deleted by name → dangling call (`list_ops.yx:53` runtime E6006 empirical
>     evidence); the fix is the restoration gate `restore_generics_with_uncovered_call_sites`, using
>     `build_call_site_map` as the single source of truth to determine coverage, the original at
>     uncovered sites is preserved. (2) The recovery segment's `HashSet` iteration order causes the
>     function table order to be non-deterministic (t2 re-review F1) → change to `Vec` and sort by
>     name, the product is byte-level reproducible. (3) The non-rewritten surface shape `TailCall` /
>     `MakeClosure` originally looked up the mapping key the same way (the comment claimed
>     conservative) → `call_form_is_rewritten` single-point judgment, non-rewritten shapes are
>     uniformly counted as uncovered. Each has its own pinned test (pre-fix red-state empirical
>     discriminating power). The corpus diff baseline is the same as the commit regeneration with
>     just one line (the `remove_at` frame supplements `(int64)`). **Commit strategy note**: when
>     typecheck collection and mono fix are split into two commits, the staged tree containing only
>     the former triggers `stdlib_docs` to go red for HEAD's existing defect (the same family
>     `list.is_empty([])` E6006)—the two halves are each other's green-light prerequisite, so they
>     are combined into a single atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closing): (1) **D2 ruling A**—user module exports
>     carry `generic_fn_type_params_snapshot()` with `Export.type_params`, isomorphic to the std
>     path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source of
>     qualified-call monomorphization, signature back-inference is downgraded to a fallback (merged
>     into R2's scope). (2) **HKT declaration side active, call side broken** (inferring a type
>     argument E1002 / explicitly giving a type argument E1010) is registered as WBS 3.5 (3
>     third-level tasks). (3) **The deferred bucket never drains under a non-generic container** is
>     merged into the acceptance sub-items of 3.4.8 ②: the bucket must drain or loudly error, silent
>     non-specialization is not allowed. (4) **Closure variable call-by-name runtime E6006** is
>     registered as WBS 3.6; semantics (i) support indirect calls / (ii) reject at compile time,
>     **already ruled (i)**—indirect call supported: the `CallDyn` mechanism is in place (the
>     closure-expression path at `ir_gen.rs:7945`), what's broken is that the `Var` callee is
>     downgraded to a name-based `Call`, and typecheck already accepts this program. (5) **Trade-off
>     criteria solidified**—option rejection reasons can only be based on correctness and
>     readability, "large change surface / new mechanism / high cost" does not constitute a
>     rejection reason, drops into `coding-rules.md` part two and the HOWTO self-check table;
>     trade-offs needing user decision are presented per `AGENTS.md` "When requesting user ruling
>     (mandatory)" with all four items (situation + evidence / per-option pros and cons / clear
>     recommendation + cost / breaking-change callout).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② landed per the #14
>     design: `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail
>     appended mono; the Linking arm aggregates the whole-program requests and qualifies
>     `containing_fn` **before taking each unit's IR** (otherwise the deferred bucket key and merged
>     IR are from different sources, the bucket never drains). **① closing (D2 ruling A)**: user
>     module exports carry a snapshot of declared type parameters via `Export::type_params`,
>     isomorphic to the std path (`yx_sources.rs:62/108`). **Existing defects empirically confirmed
>     and fixed in the process**: (a) **F1**—`qualify_names` only rewrites function bodies,
>     `ir.init` is never visited, on multi-file the function definition is already qualified while
>     module top-level calls are still bare names → runtime E6006 (**reproduces even when
>     non-generic**, unrelated to generics); the fix is for init to reuse the same `rename` /
>     `aliases` and the same `rewrite_call_names`. (b) The mono side's "whole-program unique suffix"
>     heuristic and missed-hit silent `continue` → delete the heuristic, missed-hit becomes **loud
>     E3018** (ambiguous silent mismatch is worse than no parse). **Regression caught and fixed by
>     independent verification**: t6 found that after named import `use lib.{pick}`, the bare-name
>     call was falsely rejected by E3018 (the IR instruction name is the alias qualified name, the
>     request name is the bare name, the aggregation side only qualifies by unit key) → fixed by
>     having the aggregation side **directly call ir_gen's** `import_aliases_from` and
>     `resolve_call_target` (extracted as a single pub point, one producer, one consumer), and
>     covered the named-import two-shape with a matrix pinned test. **Filed**: the init segment's
>     `DefId` gap was empirically confirmed to be a "consistency gap" rather than a live defect (the
>     codegen name fallback can resolve, four types of init calls measured pass), registered as WBS
>     3.7.4 to close the "function body goes via def, init relies on name" caliber fork.
> 18. init-segment DefId caliber fork closing (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single-point (Call / TailCall string operands + MakeClosure), the
>     function body and `ir.init` share the same implementation and the same `resolve` closure;
>     after 208 IR snapshots regeneration, only 309 pairs of `(def=_)` → `(def=dN)` changed
>     (other-lines 0; whole tree init segment 0 residual `(def=_)`). The codegen DefId main path and
>     the name-fallback two states produce translation results and execution values identical item
>     by item (four types of init calls measured), corpus baseline zero drift; the probe is flipped
>     and strengthened by the new caliber (identity + precise index + execution value, fallback init
>     coverage turns red). **Erratum (2026-10-11)**: the commit numbers above have been rewritten by
>     `git pull --rebase`—original b554497b / e60dcc87 / b7ac7396 correspond in order to the
>     post-rebase 375789d2 / d922a551 / 38bd4b9e; the pushed dev history takes the new numbers as
>     authoritative (the old numbers were local temporary numbers, not entered into the remote
>     history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④'s design basis is in #14; ②'s
>     landing is in #17, this item is its closing.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` is pure merge only (aggregate
>     requests + `link_module_irs`), the whole entry-validation block stays in `RoleClassification`;
>     `ir_generation_check` retains `us.ir` after generation (in memory only); mono failure goes
>     into the collection channel in Check, stays `fail_with` in MultiFile; `into_outcome` first
>     does `take_check_diagnostics` then judges faults (structural faults no longer clear
>     already-collected diagnostics). **Verification**: four rounds of independent verification (t3
>     / t4 / t10 + t5 cross-form matrix) and one round of re-review (t6) all pass; corpus diff
>     340/340 line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps
>     found and fixed in the same round**: F1 (rootless multi-file check's request name and IR
>     definition key are from different sources → silent non-specialization, see the WBS 3.4.8 row's
>     landing note) — fix is the aggregation side exact back-lookup by real definition key (not
>     heuristic), the first version of the fix is the aggregation side exact back-lookup by real
>     definition key (`flat_defs`); **same-day erratum**: this bridge was deleted in the regression
>     fix, the root cause is solved by `module_key`'s dual-role decoupling (`script_mode`), the
>     three are the same source `{unit-key}.{name}`. The location and fix of this regression (CI
>     doc-example gate 258 blocks, 10 failures → 0 failures) is in the WBS 3.4.8 row's landing note.
>     **Filed (existing, not a regression in this round)**: E3005 help text doesn't match
>     resource-protection semantics; multi-file E3005's span label misalignment (HEAD binary
>     reproducible); request-collection surface's type-parameter position dependency (same surface
>     goes to 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic-surface closing (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (executor-layer guard reachable)**:
>     interpretative calls changed to **explicit stack-driven**—`CallStatic` / `CallVirt` /
>     `CallDyn` / `Spawn` / `SpawnFromList` pushes a frame and hands control back to the outer
>     `step_one`, the frame carries `ReturnTarget{Host|CallerSlot|Discard}`, writes back the
>     caller's slot on return, multi-call instructions continue running with the frame's
>     `pending_calls`. Baseline measures each layer's interpretive call consuming native stack
>     **2544 B**, crash threshold at `count_down(394)` (Windows main thread 1 MiB); after the fix,
>     20 000 layers of recursion can run on a 512 KiB stack thread (about 0 per layer). The guard
>     (`push_frame`) is therefore reachable under any entry, any `max_stack_depth` value;
>     `ExecutorError::StackOverflow` changed to carry the real limit (the original
>     `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2 (deep-stack backtrace
>     truncation) was merged into this task by user ruling** (same file, not separable by file):
>     backtrace changed to first 16 frames + count omitted + last 8 frames, shallow stacks unchanged
>     frame by frame. **3.10.1 (compile-time unguarded native recursion, #447)**: first locate, then
>     change—the crash point is `MonoType::type_name` recursion (**328 B/layer**, SP probe +
>     per-operation threshold), same path has derived `Clone` (~400 B/layer), `substitute`'s deep
>     copy, `type_name_hash` (164 B/layer), `collect_generic_type_refs`; after changing the four to
>     a work list and removing one whole-request deep copy per layer, pathological generic recursion
>     with `mono.max_depth=5000` produces **E3005 (with real limit)**, no more stack overflow.
>     **3.11.1 (task-boundary error-code fidelity, #448)**: the task-result error side changed from
>     value to **structured** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), cross-boundary by-value restoration of `ExecutorError`;
>     task-internal E6001 / E6003 / E6004 / E6008 in Standard mode and Embedded messages
>     character-by-character identical. Two **pub API payload shape changes** are committed with
>     `BREAKING CHANGE` in the commit message. **Commit strategy note**: 3.9.1 and 3.11.1 **are
>     inseparable at file granularity**—`interpreter/executor/executor.rs` simultaneously carries
>     the trampoline and the error channel's producer / consumer sites, `TaskFailure` is defined in
>     `runtime/engine.rs`, staging only one side fails to compile (pre-commit with **staged subset**
>     compilation, actually rejected twice), so per the same caliber of #15 they are combined into a
>     **single atomic commit**. **Known limits and follow-up**: Drop derive (~75 B/layer) ⇒
>     `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate's load on native
>     recursion (~3 696 B/layer, default 256 layers ≈ 924 KiB), currently zero call sites,
>     production unreachable, future wiring must be closed in the same family; Standard mode task
>     interpreter constructs by default `ExecutorConfig` ⇒ its E6004 limit is always 1024 (config
>     propagation registered as WBS 3.12, user ruling B: separate team); test runtime budget opened
>     as issue #449 (pathological fixture 76s, clippy dual target, commit runs full set), not part
>     of this RFC's construction surface. **Corpus directory layering (same round, 8605a081b)**:
>     `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures split into 23
>     category-divided subdirectories, 88 IR snapshots migrated in mirror-path sync; corpus baseline
>     regenerated by the same commit (342 lines, independent re-count line-by-line comparison: only
>     the `file` field and 2 newly added entries changed), 41 snapshots regenerated because the file
>     header path got longer, line-by-line verification shows **only span offset changes** (after
>     offset-removal normalization, zero difference).

**Refactoring approach for the ten entries (function by function)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body, change to constructing `Program { kind: SingleFile, … }` and handing to Driver                                                                                                                            |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                             |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                                     |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                                       |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filter to the target file                                                                                                                                                                        |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                                           |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                                              |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (delegates to Driver)                                                                                                                                                                                                     |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project goes through `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence in the single-file branch is deleted**, and the call changes to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | The `standalone` branch (`614-616` → `check_single_file` `623-661`) is deleted, unified to `Program { kind: Check }`                                                                                                                |

**The duplication degree between `check_project` and `compile_project` (about 40% shared / 60%
fork)**. The shared part: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parse loop, per-file checker assembly, result handling. The fork: items
2/3/4/5/7/8/9/10/11 of the 11-item table above. **This 40% shared skeleton is exactly the value
range of `Driver`**—the 60% fork is entirely "different stage set" or "different aggregation mode",
and that is exactly what `Program`'s two fields can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causes this stage to be skipped" is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped` itself, the information of "why didn't I run" is scattered across 12 stages and cannot be
audited centrally. Instead, `dispatch` decides at the beginning of the loop based on the dependency
relationships:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the bug this document is meant
to fix—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip", which
produces no diagnostic, just because the obligation happens to be empty. After the rule is
established, any "because X wasn't done, Y didn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Caused by upstream `Abort`                               | "Stage Y did not execute because upstream stage X failed"                                                                            |
| Caused by prerequisite obligation being empty            | "Stage Y did not execute because no obligations to process (normal)"—**this item severity = Info, doesn't count into warning count** |
| Conditions not met (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because the configuration is not enabled"                                                                   |

The second is the key: it makes "normal skip" distinguishable from "abnormal skip" in the diagnostic
stream, and doesn't pollute the `warning_count` of `yaoxiang check` (`diagnostic/mod.rs:637-641`
depends on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                            | Current State  |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)              | Already exists |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project and single-file requests in the same process, while CLI's `run` and `check` are two
independent calls; putting them on `Program` avoids the Driver holding mutable global state.

**Note the current semantic difference between the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a matter of "early-exit
or not"—they are two different checker entries. After unification, the same implementation should be
driven by the `Aggregation` parameter, not by keeping two functions (**this is an item that this
document needs to additionally verify, see "Known Limits and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-layer impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representations introduced, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type's ownership and layering**: what migrates into the ledger is **settlement
responsibility**, not the type's owning module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the other field types stay where
they are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3's
consumers (such as `ir_gen` reading `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`**—this is consistent with the driver
no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2 / L3 / L4 must not reverse
`use crate::driver`**. A `Driver` appearing in `TypeChecker`'s imports is considered a violation,
intercepted by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Refactoring                       | After Refactoring                                                                                                                                                                                                                                                                                                                        |
| ----------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` for standalone no longer goes through this pipeline—uniformly changed to `compile_project` (MULTI_FILE stage table), Script semantics driven by rootless; this line now only describes the `lib::run_file` / `Compiler::compile` library path) |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already plugged in by P3; **warning surface stays non-additive** (decision B1, 2026-10-09: dead-code family warnings don't enter the compile path—the pool semantics is being fixed by 4.9, unified warning surface is on hold until after 4.9; the original draft's "new warning output" in this line is voided)        |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                                 |
| LSP (in-project)                    | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                                 |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                               |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above, unified                                                                                                                                                                                                                                                                                                                   |

**The only intentional behavior break** is the `.expect()` at `predicate.rs:35` being changed to
return a diagnostic. This conforms to RFC-027 §8's "not bound to a specific solver" positioning, and
is also the contract already declared by `backend.rs:60` ("the caller should degrade
conservatively")—the current implementation is the opposite of its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file / N for multi-file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                             |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead-code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **preserved**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed down to a Program constructor                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed down to a Program constructor                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` goes through Driver                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The two-choice between `check_module` / `check_module_collect_all` in `typecheck_with_registry_in` is changed to be driven by the `Aggregation` parameter                                                                                                                                                                                                                  |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | The manual stage sequence in `run_diagnostics` is deleted, and the call changes to Driver                                                                                                                                                                                                                                                                                  |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` uniformly goes through `Program { kind: Check }`                                                                                                                                                                                                                                                                                            |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                            |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falling to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | The empty `ProofResult::Unproven { .. } => {}` arm changed to produce diagnostic (**fix the second silent-drop point**)                                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | The `unwrap_or_default()` degradation path adds a warning diagnostic                                                                                                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix the comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer-order table changed to **actual** execution order; remove "lower-layer failure prevents upper-layer from running" (no short-circuit exists)                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: new process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumers `predicate.rs` / `checker.rs` / `ownership.rs` are changed to go through the shared port. The `&'static` raw-reference shape is infeasible (`dyn Solver` is not `Sync`), the original text is governed by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct—**the problem is at the consumer side, not at the producer side**, changing
the producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (the reachability judgment of wasm branches goes to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                   | Compatibility                                                               | Disposition                                                                                                                                      |
| -------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Multi-file path adds proof_execution                     | **Breaking**: previously silently-passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; staged release, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                   | **Breaking**: programs that previously compiled now produce warnings        | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                         |
| `assert_drained()` first run produces W-level diagnostic | **Breaking**: diagnostic set increases                                      | W first then E, two steps in S2/S4                                                                                                               |
| `predicate.rs` panic changed to diagnostic               | **Improvement**: no longer crashes                                          | No breakage                                                                                                                                      |
| `build` / `dump_bytecode` subcommands                    | **No impact**                                                               | These two paths don't go through proof_execution                                                                                                 |
| `TypeCheckResult` field migration to `Obligations`       | **Internal refactoring**                                                    | If there's external dependency on the `pub` API surface, it needs to be synced; in-repo consumers are fully listed in the change list            |

---

## Implementation Points

This document corresponds to **P3 (fix correctness vulnerabilities)** and **P4 (stage contract and
unified Driver)** in RFC-039's global stage sequence. The following S1–S5 is this document's
internal implementation order, **and each step's acceptance criterion references the C2 category of
the [Equivalence Criteria Document](07-equivalence-oracle.md)** (orchestration changes: the
**diagnostic set** at each entry is the same + corpus behavior is the same).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written as failing first.

| Deliverable                                                                                                                                               | Acceptance                                                                                                    |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                             | **Must be red**. If accidentally green, the vulnerability analysis of this document needs to be re-reviewed   |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48—don't mix into the single-file corpus tree; project fixture with `yaoxiang.toml`) | Diagnostic sets of the two versions (single-file and multi-file) corpus can be compared                       |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                            | Marked `#[ignore]`, S4 turns green (red state archived: after removing the sole settlement point, `pairs: 0`) |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-identical**                                |
| Whole corpus (293 `.yx` files) diff                      | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately fixes no bug**—it just moves existing behavior into Driver. The acceptance
criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, restoring `pipeline.rs` is enough.

### S3: Merge the Four Orchestrator Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The diagnostic sets of the four entries, normalized by `Aggregation`, are the same  |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behavior consistent in and out of project                          |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                          |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                                 |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No more panic without Z3                                                            |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If the
structural merge has a problem, only the structural commit can be rolled back, the vulnerability-fix
commit is kept—at this point `test_multifile_proof_obligation_not_dropped` stays green; the reverse
(keep structure, roll back fix) would turn red, which is an unacceptable intermediate state, and is
forbidden to merge.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                                                                                                                                 |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 test cases in `src/driver/tests/obligations.rs` turn green (for implementation form, see §2 revision note; the test count is governed by that file) |
| `scripts/ci/check-obligations.py`            | Static gate in effect                                                                                                                                      |
| Obligation diagnostic W → E upgrade          | Whole corpus diff, then manual review of each new E                                                                                                        |

**Rollback point**: The severity of `assert_drained()` can be controlled by a configuration item;
W/E switching doesn't require code structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                                      | Acceptance                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The layer order in `layers/README.md` is changed to actual order                                                                                                                                                                                 | The document corresponds to `checker.rs` call sites item by item                                                                                                              |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter the cache**—otherwise one timeout is solidified in the process-level cache across compilations, and pollutes the `cargo test` thread-shared state) | Production path `default_solver()` call sites go to zero (only tests preserve); cache hit rate is observable across the three consumers (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                                  | Diagnostic when there is no `body_checker`                                                                                                                                    |
| Reachability judgment of the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                             | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the judgment need                                                                                  |

**Note**: correcting the layer order will change the diagnostic set and may expose a large number of
previously silent `Unproven`. **This item should be independent of S1–S4**, and not mixed into the
same PR as the entry merge.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                                            | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage model**          | `Stage` is a compile-time exhaustive enum of 12 variants, **refuse runtime registration**                                                                                                                                                                                                                                                                                                                                                                | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant, `dispatch` fails to compile. This is isomorphic to the mechanism that the opcode table has been verified in RFC-039 routing table B. `StageScope` further turns "per module or project level" into a type fact, eliminating the call-count question that requires human reasoning                                                                                             |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, used in parallel with the **static** gate of `scripts/ci/check-obligations.py`; **terminal severity E-level** (2026-10-13 user ruling A: same code upgraded to E, code number unchanged; **for implementation form, see the §2 revision note**: ledger doesn't copy field structure; red list—0 entries after cleanup—mechanism preserved, does not enter user-visible diagnostic stream) | Fixes a whole class of bugs, not just one: the same type of latent risk has at least 2 places currently verified (`proof_calls`, `checker.rs:1313`), all 13 obligation fields are within range. W→E in two steps is to keep every step's C2 criterion usable—now the obligation list has been cleaned to 13 entries, the red list is zeroed, the healthy state emits nothing, so the terminal state directly takes E (see the §2 revision note parts VII and VIII) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **don't touch** the producer-side logic of the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448`                                                                                                                                                                                                                                                                                    | The problem is at the consumer side, not at the producer side. Changing the producer side would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                                                                                                          |

This design additionally eliminates two classes of cracks:

- **The crack between "comment promises" and "code behavior"**. The "silent pass must not be
  resurrected" at `checker.rs:5165` and the "lower-layer failure prevents upper-layer from running"
  at `layers/README.md:3` are both **comment-level contracts**—the former is fulfilled by
  `assert_drained()`, and the latter is exposed by topology-driven `Skipped` diagnostics.
- **"Forgot to append to an array"**. Currently five functions each hand-write their call sequence;
  after unification, there is a single stage table, and new stage missed-hookups cause compile-time
  errors.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—of the 11 inconsistencies, 7
  (two implementations of dead-code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type handshakes**, they're a configuration question of "do you
  want to run some analysis"; it also can't express `Aggregation`, and can only move the two-choice
  of `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancels compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, equivalent to
  replacing "five functions each hand-write their calls" with "forgot to append to an array", which
  is exactly the cause of the current 11 inconsistencies. This repository already has same-shape
  anti-examples: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implementation.
- **Only patch the bug, don't touch the structure**—can fix the known 2 bugs, but only covers 1 of
  the obligation fields, `Stage` is still scattered across 5 functions, and the next fork point will
  keep growing from here. **It must be done first** (as part of S1/S3), because structural
  refactoring needs a known-red test to prove the criterion is effective.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a general entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **wholly** discarded. This is looking for a bug at the wrong level:
  the bug is at the orchestration layer, and detection must be at the orchestration layer.

## Known Limits and Risks

- **`orchestrator.rs` slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is a common cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously silent issues"**. After the fix, there may be a wave of "new error" user reports—they
  are real bugs that have just never been reported. This must be clearly stated in the CHANGELOG.
- **The severity switch of `assert_drained()` requires per-field manual judgment**. For some
  obligation fields (e.g. `module_namespaces`), "no consumer" may be by design not needing
  consumption, and shouldn't be warned. S4 needs to go through them one by one, and can't be a
  one-size-fits-all.
- **The `Program` abstraction may be premature**. If the stage set for some entry is unstable for a
  long time, `stages()` will degrade to "pass a different array every time" free parameter, and the
  contract constraint will fall flat. **The mitigation is `test_program_stage_coverage` asserting
  that `stages()` can only come from the 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability-fix criteria depend on the
  [Equivalence Criteria Document](07-equivalence-oracle.md)'s vulnerability tests being red first.
  **If that document does not establish the multi-file corpus layer first, S1 cannot be
  accepted**—because the existing 293 corpus files all take the single-file path, with zero coverage
  for this class of defect. This document does not involve the IR verifier (`verify_loose`); that
  part of the prerequisite work is out of scope of this document.
- **Layer-order correction will expose previously silent `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership`'s order is opposite to the declaration). This is a
  diagnostic set change risk, and should be done independently.
- **Thread safety after SMT backend is changed to singleton is undetermined**. `backend.rs:13-19`
  already says that `Solver` is only `Send` but not `Sync`, and `Z3Backend`'s cache is `RefCell`;
  after changing to a singleton shared across compilation units, it must be confirmed that all
  access paths go through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). Should be opened after the equivalence criteria stabilize.

> **The open questions originally listed in this section have all been ruled on.** Decisions are in
> the [RFC-039 rulings registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth set)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 grading, three-tier criteria, the
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Source design of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — The
  reference code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "silent pass must
  not be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — The explicit declaration of "lower-layer
  failure prevents upper-layer from running" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — The explicit reason for
  SMT-initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "the caller
  should degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — The 13 obligation fields that downstream must
  consume (after 3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The single-test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path, hence
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — `cdylib` + `wasm-bindgen` in the shim crate, not the main crate
- `build.rs:19-55` — Error-code build-time gate, an example of this project's enforcement mechanism
