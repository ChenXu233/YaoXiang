# Compilation Stage Contract and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039: Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> The four-layer model, the acceptance-criteria grading, and the execution-stage sequence are in the
> RFC-039 main text; the positioning of each subsidiary document is in
> [this directory's index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
problem: **"Which stages ran in this compilation, which didn't, and why" is currently scattered
across six paths and ten entry functions, each hand-wired — there is not a single place that can
answer this question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are violated by the architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally says "Unproven → compile
error, no degradation, no silent pass… must not let silent pass be revived." But this invariant only
holds on the **single-file path**: refinement constraints on proof functions, of the form
`x: Sorted(3)`, **silently pass** under the three paths of multi-file `yaoxiang run`,
`yaoxiang check`, and LSP — the proof function is never executed and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, not registered at runtime) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-function refactoring for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency direction conventions, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1–C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- The global P1–P10 execution order and G1–G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and the **order** to do it; this document gives L1's
**concrete form**, the **per-file change list**, and the **implementation phases within this
document** (corresponding to RFC-039 global sequence P3 "fix correctness vulnerabilities" and P4
"stage contract and unified Driver"). Where this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are executed per the **C2 (orchestration change)** category of the
[Equivalence Criteria document](07-equivalence-oracle.md): same diagnostic set at each entry + same
corpus behavior.

## Current State

> All items in this section are **verified facts**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries are the Only "One Miswrite, Silent Whole-Repo" Structural Defect

It is qualitatively different from the other two defect types (module boundaries, test wiring):

| Defect                  | Typical manifestation               | Is there a signal?   |
| ----------------------- | ----------------------------------- | -------------------- |
| Lexical/syntactic error | Source code is wrong                | Has diagnostic       |
| Type mismatch           | Type is wrong                       | Has diagnostic       |
| Stage mis-wired         | Some stage was never called         | **No signal at all** |
| Obligation not consumed | Field is filled but no one reads it | **No signal at all** |

The common feature of the latter two is: **failure does not produce an error**. Therefore they
cannot be solved by "writing code more carefully" or "reviewing more strictly" — code review can
only see what was written, not **what was not written**. This is exactly the root cause diagnosed by
RFC-039: "this project treats 'design' as a documentation convention, not as an executable
constraint."

### Existing Enforcement Mechanisms in this Repository

The same repository already has mature enforcement mechanisms; they just haven't been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are **hard-gated at build
  time** by `build.rs:19-55` via `tools/code-tables`, comparing each entry against the RFC-013 code
  table; any inconsistency immediately `panic!`s and refuses to compile.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` is 6227 lines, 47.9%) has high test
  density and a declared wiring for each module's test subtree — this is the most complete
  test-wiring area in the whole repo, and can serve as a formal reference for stage-layer gating.

**The design capability is sufficient. What's missing is "putting a gate of the same level in the
orchestration layer too."**

(Line-count basis: `(Get-Content).Count`, see the "Line-count basis" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry function                                        | Location                                      | Which compilation does it take?                 |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5-stage direct call                             |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking          |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics     |
| 3c  | **LSP in project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR        |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1               |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                   |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project takes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                   |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1        |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call site     | Implementation                                                       |
| ------------------- | ------------- | -------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                        |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined within** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The table below gives evidence cell by cell. **A blank does not mean "this entry doesn't do this
thing", but "there is no code in this entry that does this thing"** — which is the essence of the
defect.

| #   | Stage / behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **yes** `187-203`                                   | **no**                                                    | **no**                                                      | **no**                             | **no**                                |
| 2   | Dead-code analysis                                       | yes `275-278` (gated by `config.dead_code.enabled`) | **no**                                                    | yes `356-377` (role-aware, no config gate)                  | **no**                             | **no**                                |
| 3   | W1006 local module shadowing                             | **no**                                              | **no**                                                    | yes `336-348`                                               | **no**                             | **no**                                |
| 4   | W1003 unused import                                      | yes (`277` collects `type_result.warnings`)         | **collected but never output**                            | yes `350`                                                   | **no**                             | **no**                                |
| 5   | W1001/W1002 dead-code family                             | yes (same as 2)                                     | **no**                                                    | yes (same as 2)                                             | **no**                             | **no**                                |
| 6   | **Monomorphization**                                     | yes `381-389` (gated by `config.mono.enabled`)      | **no**                                                    | N/A                                                         | N/A                                | **no**                                |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, drops `used_by`/`shadow_events`)       | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **no**                                              | **no**                                                    | yes (`321-328`)                                             | **no**                             | **no**                                |
| 10  | Global slot allocation                                   | N/A                                                 | yes `allocate_global_slots` (`147`)                       | **no**                                                      | **no**                             | **no**                                |
| 11  | IR generation + qualified-name rewriting + linking       | yes (`205`)                                         | yes (`152-236`)                                           | **no**                                                      | **no**                             | yes (independent ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell check)**: this table is the diagnostic snapshot
> of the `9e02e4db` baseline, kept as-is. The current-state differences after P3 lands:
>
> - **Row 1 fixed**: proof_execution is shared across five entries via the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the single-consumer defect
>   no longer exists.
> - **Rows 2/4/5 unchanged** (multi-file `run` still doesn't output W1003, doesn't run dead-code
>   family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file only) → assigned to WBS 3.4.8 (prerequisite 4.1.3,
>   unverified potential risk).
> - **Row 7**: `check_module` / `check_module_collect_all` dual entries → assigned to 4.2.7
>   (Aggregation-parameter driven).
> - The "Ruling #434" referenced by code previously had zero registration in docs — now registered
>   as RFC-039 **D57**.
> - **Embedded std included in asymmetry (new fact outside the table, registered 2026-10-09)**: the
>   single-file path unconditionally injects `std.list` via `merge_embedded_std_ir` (required for
>   `for`-loop desugaring, #117 hard switch); multi-file `discover` previously only recognized
>   explicit `use` — in-project `for` loops compile cleanly, then E6006 at runtime (probe-verified).
>   Fixed and struck through WBS 4.10.2; the same-family "multi-file missing the single-file step"
>   parse-hard-abort quirk is filed under WBS 4.10.1 — **also fixed** (landed right after 4.2.2; the
>   Check path degraded to per-file collection; from this audit's "stage coverage / field
>   consumption" view, this is a compilation-unit membership difference, a missed dimension).
>
> - **Embedded std registration surface coverage (found during 4.2.5 construction, fixed)**: the
>   Registry arm re-harvests registration for embedded-std units via `extract_module_info`,
>   overwriting the native half already registered by `with_std()` under "native + yx surface merge"
>   (lost `result.is_err` etc. → std.test false-positive E1043). The third missed dimension of this
>   audit: "the merge semantics of two registration sources for the same module key" — the
>   single-file path's registration table is only shaped once via `with_std()`; multi-file/Check's
>   Registry arm inserts per unit, which is the only place this surface exists.

**Two places need precise wording, or you'll write them wrong:**

- **The exact conclusion for rows 4/5 is**: `yaoxiang run` under the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; under the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138` — `type_results.push(result)` stores the full `TypeCheckResult` (including
  `warnings`), but this function **has no `result.warnings` read site at all**; the only downstream
  use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion also has different sources at each entry**: `check_project:385-395`
  uses `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`,
  only judges "is there a manifest"). The two functions give different answers to "which files must
  define `main`."

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are reproducible.**

**Step 1 — The obligation production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the whole
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

The semantics: "the predicate arguments of this refinement constraint are compile-time literals;
this proof function must actually be executed to decide."

**Step 2 — The consumption point is inside checker, but it only emits, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                              | Location                                                                                                     | Behavior                |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter-position refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **emits no diagnostic** |
| Call-site argument check            | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostic** |
| Return-position obligation          | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real line of defense, not a forward-looking stopgap: must not let silent pass be revived.`

i.e.: the author knew that "producing no diagnostic" is a defect, explicitly classified it as a
**known, homeless intermediate state**, and designed under the assumption that "someone will come to
read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects; `checker.rs:1439` writes it as
`proof_calls, // Phase 2.5 pre-register proof function obligations` into `TypeCheckResult`. The
field is defined at `types.rs:29`.

**Step 4 — There is only one read site in the whole repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code **is read at exactly one place: `src/frontend/pipeline.rs:187`**
(passed as arg at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repo fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries skip that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) in
`src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**, but directly call
`TypeChecker::check_module` (`124` / `493` / `1386`). Thus they can't even reach that one read site
at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations also go through the
discard path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std module itself are
likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) **compiles cleanly, runs cleanly, produces no diagnostic**
under the multi-file / check / LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There's a second silent-discard point inside the layer.**
`checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` in the ownership-check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the `5164`-branch commitment "must not let
silent pass be revived", and is **independent of the orchestrator issue** — even if the entry layer
were fully fixed, this one would still be silent. (Processing timing: should be handled in the same
batch as the obligations ledger, because it's the same kind of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty — for example `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping right before `pipeline.rs`**.

The test file's own doc comment already says this. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof call (E4018) is executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx level`

**This is exactly the shape of the problem: the test verifies "the obligation is filled", but the
bug is "the consumer doesn't read it."** A test that only tests the production side and not the
consumption side is naturally immune to this class of defect.

### A Stronger Finding: the Equivalence-Criterion Main Corpus is the Single-File Path

The [Equivalence Criteria document](07-equivalence-oracle.md) takes end-to-end diffing of the 293
`.yx` corpora in `tests/yaoxiang/` as the **third-tier criterion**, as the main acceptance means for
C2 stages. But empirically:

- The `tests/` directory has **no `yaoxiang.toml` at all** (zero hits by glob in the entire
  directory).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, none
  cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is
precisely empirical evidence of this. That file is tagged `// expect: compile-error E4018` (line
15), and its header comment on line 7 says "Status: ❌ Should be rejected at compile time" — **it
passes precisely because it takes the only path that actually executes the proof function**.

**Conclusion: the third-tier criterion needs a multi-file corpus layer added, otherwise it cannot
serve as the acceptance tool for this document.** See "Implementation notes" S1.

### Obligation Fields: 13 "Produced-then-Contract" Fields, Zero Mechanism

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, there are **13 fields** whose
doc comments explicitly say "produced by stage X → consumed by stage Y", i.e. they are **cross-stage
obligations** by nature:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Row-fetch convention**: definition line = the **declaration line** of the field in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of the doc comment — the comment block for the
> same field may span multiple lines, and the declaration line is the only uniquely anchorable
> position. The **struct range** in the sentence above the table
> (`src/frontend/core/typecheck/types.rs:16-76`) floats with the struct's line numbers, mechanically
> reconciled by gate **F5** (wrong write turns red); the definition-line column is reconciled by
> **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | | `release_plan` | `38` |
> ownership → IR generation (**`PlanId`-keyed**: read at `ir_gen.rs:360`
> (`type_result.release_plan.clone()`), used at `:3103` (`ReleasePlan::consume`);
> settlement-complete marker at `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId`-keyed** — call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-complete marker at
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Ruling record (3.4.9, user ruling A on 2026-10-13)**: the original row 14
`interface_impl_registry` (`types.rs:49`) has been **reclassified as an in-tree fact** — by design
it's consumed in-phase by typecheck and has zero cross-stage reads, so it is not an obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), seven constraint-solving queries in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), `checker.rs:214-217`
  read-only accessor, `checker.rs:974` injects into body_checker.
- **Zero cross-stage reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; the `src/driver` side only has the
  ledger's own registration and docs (already removed this round).
- **Disposition**: remove the doc-comment **declaration** "mirrored for middle-side / LSP
  consumption", keep the field and its in-phase consumption; the field is registered in
  `NON_OBLIGATION_FIELDS` of `src/driver/obligations.rs` (with reason). If LSP really needs that
  table later, register under 4.6.x, **do not create a consumer in this round**.

**All these fields cross layers keyed by `Span` or by name table.** The Equivalence Criteria
document has already noted that a span-key contract mismatch will "silently fail, no error at all"
(`ReleasePlan` is typical: any inconsistency in how the two sides compute the span → `Drop`
instructions silently vanish).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
coincidental**:

- `release_plan`'s consumer `ir_gen.rs:360` (`release_plan: type_result.release_plan.clone()`) lives
  inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the full
  `&TypeCheckResult` to it** — the consumer is in the downstream module, and the downstream module
  is on the inevitable path of every entry.
- `proof_calls`'s consumer is at the **top of `pipeline.rs`**, which is **another entry
  implementation**. The orchestrator's four entries don't even pass through that layer of
  `pipeline.rs`.

**Verified fact**: the repository currently has **no mechanism whatsoever** to ensure these 13
fields are consumed. The only "protection" is that `ReleasePlan` happens to hitch a ride on IR
generation.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree-Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: the fields in `TypeCheckResult` that are not cross-stage
obligations, registered one by one (field name + reason), **complementary to** the "§Obligation
Fields" table above:

| Field                     | Why it is not a cross-stage obligation (key point)                                                                                                 |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit-identity meta info (0 hits in `src/middle`)                                                                                                   |
| `diagnostics`             | Error-diagnostic channel (existing channel, not "produced-then-contract")                                                                          |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but doc comment doesn't declare a cross-stage contract ⇒ not enlisted                               |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings` — no declared contract                                             |
| `semantic_db`             | Consumer is in LSP (since 4.6.1 sent out via the `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`), not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`, doc comment doesn't declare a contract                                                                |
| `warnings`                | Warning-diagnostic channel (same kind as `diagnostics`)                                                                                            |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption sites in "§Obligation Fields" ruling record)                                               |

**Why not "wire a consumer"**: creating downstream consumers for these fields would only invent fake
obligations, dressing up "by design doesn't need cross-stage consumption" as a contract defect; the
correct closure mechanism is **reverse containment** — the `TypeCheckResult` field set = obligations
list ∪ non-obligation registration (hard-gated by `scripts/ci/check-obligations.py` criterion 1: add
a field and forget to register, remove a field and forget to sync, non-obligation registration
without reason, two tables overlap → all turn red on the spot).

### Four Additional Contract Defects in the Proof Layer

All of the following come from proof-layer analysis, are independent of the entry-bifurcation
problem, but all belong to the category "stage contracts are not enforced."

**(a) Layer order is declared only in docs, no mechanism enforces it; the `equivalence` layer is not
in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (anchored by function name / drive point, line numbers taken from
the current tree):

| Actual order | Call site                                                                                                                                          | Corresponding layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement-walk driver `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                    | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                       | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not in `layers/`)                                                                                  | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                             | `equivalence.rs`    |

Layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) only exist in
each module's own header comments, and don't match the call order; there's no "number → call timing"
map in the README.

`equivalence` is only used by `inference/assignment.rs:17` as the `is_subtype` utility function,
unrelated to `ProofResult`. At the same time **there is no short-circuit whatsoever** —
`check_module_impl` keeps going down after `add_error`-ing each termination error one by one; "the
lower layer failing means the upper layer won't run" is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: the hard-failure `panic` was eliminated by 3.3.1 of P3 (the SOLVER
> slot is `Option`-ified; missing falls back conservatively to `SMTResult::Unknown`); the "silently
> skip (don't inject)" was supplemented with a W1081 signal by 3.4.2. The three-shape unification
> (singleton) is executed per **RFC-039 D58**; the real fix location is `proof/smt/backend.rs` (new
> process-level shared singleton + `with_shared_solver` closure access), and the 02 change-list's
> `default_solver() → &'static` wording follows D58 (`&'static` bare reference is infeasible:
> `dyn Solver` is not `Sync`).

| Consumer                                                | Acquisition strategy                                     | When the solver is unavailable                            |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Injection at construction via `with_solver_owned`        | `None` → **silently skip** (don't inject)                 |
| `ownership.rs:627` (back-edge cut decision)             | **Each call** to `default_solver()`                      | `None => return false` (`629`) → conservatively don't cut |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "keep initialization failure as
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'." But the contract docs at `backend.rs:60` say "`None`: backend unavailable… **caller
should degrade conservatively**." **Two philosophies coexist in the same code, and the hard-failure
one panics rather than returning an error.**

**(c) A new Z3 context is created for every back-edge decision; the cache is useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge-cut decision. While
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
new Z3 context. The cache field on `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **per-instance**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`." Not shared across calls ⇒
**that cache can never hit under this call pattern**.

(For comparison: `predicate.rs:34-36`'s `LazyLock` is a real singleton. Same backend, two lifetime
strategies.)

**(d) Checker silently degrades + comment doesn't match reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, the ownership check receives an **empty type ledger and empty call
table** and runs as normal — ownership analysis degenerates to "nothing conflicts", **with no
warning whatsoever**. (Suggested fix: should be recorded as a warning-level diagnostic, or at least
leave a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after typecheck, before constraint
solving". In reality `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at
`1256`, ownership is after `1296`, i.e. what the comment calls "before" is actually "two layers
later"**.

### wasm Current State: carried by the shim crate, the 27-file branch is live

The wasm target **is built and in CI**, and the `cdylib` is not in the main crate but in a separate
shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has a wasm target dependency section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                   |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetches `libz3.a` from a fixed URL, degrades to warning on miss)                                                                                                                                                               |
| **27 files** contain the `wasm32` literal         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are mentioned in docs only                                                                                                                                       |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                        |

**Conclusion: those 27 files' `#[cfg(target_arch = "wasm32")]` branches are load-bearing.** They
decide which APIs of the main crate the wasm target can call from `wasm/src/lib.rs` (73 lines) —
`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` entirely (all three need
`std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Path that consumes `proof_calls`           | Does it execute the proof function? |
| ------------------------------------------ | ----------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                             |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                             |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                             |
| Multi-file `run` (`compile_project`)       | **No**                              |
| `check` (`check_project`)                  | **No**                              |
| LSP in-project (`check_source_in_project`) | **No**                              |

i.e.: **the existence of `wasm/` turns "proof_execution has only one consumer" into "it has
three"**, but all three are on the single-file-path side. The Driver's `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" §3), otherwise the unified Driver will miss this path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — is filed under `06-cleanup-inventory.md`. The RFC-039 open question "build a
wasm target, or delete these branches" **has expired**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Registered at Runtime)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason: Rust's exhaustive `match` can enforce at compile time that "a new stage must be handled
by the orchestration layer" — this is the same mechanism proven effective by the opcode table
(RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor-consistency check          Project
    Discovery,          // File discovery                    Project
    Parsing,            // Lexing + syntax                   PerModule
    Registry,           // Module-registry construction      Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type check (with inlined proof layer) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis         Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation            Project
    IrGeneration,       // AST → ModuleIR                    PerModule
    Monomorphization,   // Monomorphization                  Project
    Linking,            // Cross-module linking / IR merge   Project
}

impl Stage {
    /// All stages. When a new variant is added, both this array and the exhaustive
    /// match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs
once at project level) turn "which stages must run per module and which must run once at project
level" into a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once
in `dispatch` — this eliminates questions like "should `allocate_global_slots` be called per file"
in `orchestrator.rs` today, which currently require human reasoning.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on the order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the draft, corrected by
> implementation evidence —
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the draft order contradicts the data
>    flow.
> 2. The Check form's stage table is normalized by `Stage::ALL`'s topological order to **dead-code
>    first, then proof**: `check_project` currently runs proof before dead code; the two have no
>    data dependency, and the diagnostic sets are identical (C2 set semantics), only the intra-file
>    diagnostic order is normalized. The single-file path (dead code inlined into typecheck, before
>    proof) already follows ALL order, so byte-by-byte acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry` / `RoleClassification` (C3, user ruling 2026-10-09):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    draft order would force the Registry arm to "hide parse", making the stage table lie about the
>    data flow; and multi-file parse thereby drops from 2 times to 1 time. The current state is also
>    honestly registered: multi-file path parse error used to be a hard abort
>    (`build_registry_from`'s `?` propagation, the tension with CollectAll semantics registered as
>    WBS 4.10.1) — **4.10.1 is now fixed** (2026-10-09 user ruling rust-style collection semantics +
>    plan B): the Check path's parse failure degrades to per-file diagnostic collection, the sick
>    file exits the compilation unit (does not enter registry, importer reports E5001); the
>    MultiFile path keeps the hard abort (a bad file under FailFast can't produce IR, which is
>    correct semantics, nailed long-term).
> 4. Check form landing (4.2.2, user ruling 2026-10-09), two additional registrations: a.
>    Data-dependency edges 14→16: `RoleClassification` consumes Discovery's `used_by` edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadow events — the draft missed these (these
>    products in the orchestrator era came from `discover_with_used` and were discarded by the
>    `discover` wrapper; the data flow was lost the moment it didn't enter the table); b. The
>    intra-file diagnostic order normalization is extended to the complete form of note #2: the
>    E3020 entry check is moved into the RoleClassification arm (Check has no Linking stage, the arm
>    holding surfaces and AST is responsible for the check), the intra-file diagnostic order = stage
>    topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is
>    unchanged (C2 set semantics), only the stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no one consumes" from undetectable into a compile-time- or
test-time-assertable fact. RFC-039 has already listed this as "the highest-value item."

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
    // …the rest of the span-keyed fields
}

impl Obligations {
    /// Settle: called once at the tail of the stage table.
    /// Each unconsumed field produces one W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                   | Criterion                                                   | Disposition                                                                     |
| ------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Obligation is read by its declared consumer | The field is `take()`-ed / marked consumed in `Obligations` | Pass                                                                            |
| Obligation non-empty but no consumer        | Field is non-empty and not consumed                         | **Produce diagnostic** (W-level by default; `strict` mode escalates to E-level) |
| Obligation is empty                         | Field is empty                                              | Pass (no consumption needed)                                                    |

**Why start with W-level instead of E-level**: fixing obligation discard will **change the
diagnostic set**. The C2 criterion requires "same diagnostic set at each entry"; if we jump straight
to E-level, `yaoxiang check` will suddenly emit a lot of `Unproven` diagnostics that were previously
silent. Two steps (W observation first, then escalate to E) keep every step's criterion usable. See
"Implementation Notes" S4.

**The only call site of `assert_drained()`**: in `Driver::run`, at the tail of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third-file read → fail). `assert_drained()` is the **runtime** gate; the script is the
**static** gate; they complement each other; both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: this note registers the
> **implementation form, the measured numbers, and the 4.3.2 interface** of the sketch above and the
> three-tier table, item by item corresponding to code file:line. The landing commit is `1941cc47a`
> (verification tree `99079bc52`). It is simultaneously the **third** of the red-list "no silent"
> three places (the other two: the audit surface, the static gate).
>
> **I. Landing artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (`:201`, field name /
>   consumer stage / red-list), **non-obligation field registration `NON_OBLIGATION_FIELDS` (`:150`,
>   8 entries with reason)**, delivery-marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), outbound emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer-side gate `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code-string
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **The only settlement point**: at the tail of `Driver::run`'s stage table, before
>   `into_outcome()` produces `CompilationResult` — `src/driver/mod.rs:110`
>   (`State::assert_drained`, obligations.rs:570). The early-exit path of `Aggregation::FailFast` is
>   settled the same way (the books must balance).
> - **Seven delivery points** (one per arm; remove any one and the runtime W1090 must fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only outward-read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — four places with
>   the same delivery port: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single-file direct read of
>   request table) / `:1221` (MultiFile·Check: aggregated in Linking arm, `driver/helpers.rs:295`
>   reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation equation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, reason written just above at `:26-31` — the settlement granularity is "unit
>   × obligation", with no user-pointable source location, so no dummy span); six-language copy in
>   `locales/*.json`. **Emission level**: since 4.3.2 it is **error** (same code, escalates to E,
>   see the third section) — decided by `to_diagnostic` (obligations.rs:444) explicitly calling
>   `.severity(Severity::Error)`, the code number still in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, includes `--selftest`).
>
> **II. Per-cell landing of the three-tier table + two extra tiers**
>
> | The three-tier table situation above     | Implementation criterion                                                                 | Code                   |
> | ---------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by its declared consumer | **Delivery marker exists** (marker ↔ real read reconciled by static gate, see section V) | obligations.rs:618-623 |
> | Obligation non-empty but no consumer     | Non-empty + consumer stage ran + no marker → W1090 (**error level**, see section III)    | obligations.rs:624-630 |
> | Obligation is empty                      | `Obligation::is_produced` per-field empty check → pass                                   | obligations.rs:587-591 |
>
> Two **extra tiers** (2026-10-13 user ruling, not in the sketch):
>
> 1. **S1**: the consumer stage of an obligation is not in the stage table of this `ProgramKind` →
>    **don't adjudicate debt** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: consumer stage skipped with `SkipReason::UpstreamFailed` / `ConfigDisabled`
>    → don't adjudicate debt (obligations.rs:598-602). `NoObligations` does **not** exempt — if the
>    obligation is non-empty and that stage claims no obligations, that itself proves the books
>    don't balance, must report.
>
> **III. Outbound emission caliber (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: this mechanism only emits **W-level** (W1090); **no strict
> switch**. **4.3.2 (end state, user ruling A on 2026-10-13)**: **same code, escalate to E** — code
> number `W1090` unchanged, emission level raised to `error` (explicit `.severity(Severity::Error)`,
> `to_diagnostic` obligations.rs:444); **no new E code** (one fact two codes = parallel
> representations). **Escalation only changes the emission level, not the emission condition**.
> **This is a D-class breaking item of D59 (changes externally observable behavior), registered
> as-is**: a healthy program emits zero (when all obligations have delivery markers, nothing is
> reported), so existing corpora and user programs are unaffected (342/342 "diff: empty"); but once
> a run truly hits an undelivered obligation, that diagnostic is **counted in `error_count`,
> rendered as `error[W1090]`, non-zero exit code** — different from the previous "W-level doesn't
> block compilation". The caliber of decoupling code family from emission level has been registered
> in RFC-013's "emission channel" section (the code-number prefix only determines number ownership;
> the level is decided by the explicit severity at the construction point). Mechanism facts
> (measured): severity takes the explicit `.severity()`, otherwise "code starts with `W` → Warning"
> (`codes/builder.rs:186-203`); the registry's `ErrorCategory` does **not** participate in severity;
> the exit code is decided by `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and
> `--deny-warnings` (`src/main.rs:567-568`); **W1090 goes through `error_count` (since 4.3.2)
> because of the explicit error**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain ruling C on
> 2026-10-13; the current red list has zeroed, see section IV), the reason is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is filled in by
>   typecheck's pre-table injection, **always non-empty**, so per-compile it emits a **constant
>   signal** (zero discrimination), yet breaks three existing external contracts — "clean program
>   zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing cases turn red**, of which **8 are on
>   out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; another 4 in
>   `src/driver/tests/`), the corpus baseline has 24/342 fixture changes, and the only feasible fix
>   is "filter W1090 out of the zero-diagnostic assertions" = relaxing existing criteria.
> - Ruling B (downgrade to Info) still breaks 9/12 of the list-style assertions, and Info is not
>   "emitted by ruling level".
> - Ruling C measured: existing cases **zero changes**, corpus baseline **zero changes** (342/342
>   "diff: empty, behavior equivalent"), runtime W1090 becomes a **discriminating regression
>   detector** — a healthy tree stays silent the whole way, only fires when a delivery marker is
>   removed (three delivery points each had a remove-marker mutation; CLI `check` with the marker
>   removed in the Check arm printed 2 lines of `warning [W1090]`).
>
> The "no-silent" of the red list is carried by three places: ① the
> `DriverOutcome.obligations.findings` audit surface (carried out every run, with WBS id and
> evidence pointer); ② `check-obligations.py` prints the red list (currently empty) + staleness
> check on every run; ③ this note.
>
> **IV. Red-list current state (cleared) and the landing records of 3.4.4 / 3.4.5 / 3.4.9**
>
> Red list 3 → **0** — the three clearing tasks all landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared duty ("ir_gen produces
>   CreateVariant") is equivalently fulfilled by an in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST shape
>   `FieldAccess{Call(Var)/Var}` + takes declaration order from `sum_type_variants`), the call site
>   at `:7496` replaces the whole Call with `CreateVariant`; the function's doc explicitly says
>   "span-key cross-system is unreliable (the two ends have different semantics for the Call node
>   span), so detect in-place" — **wiring a consumer would bring back the span-mismatch defect
>   that's already been fixed**. Deletion surface: `types.rs` field, the collection at
>   `checker.rs:1489-1493/:1565`, the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`, the `VariantCtorCall` struct in `environment.rs`; the type inference
>   and all diagnostics of `infer_variant_ctor_call` are **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (keep the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: write in phase
>   (`environment.rs:205-219` registered in same order as `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550` decision point, the unique-hit case writes to `overload_resolutions`
>   at `:4550`); what is deleted is the **zero-reader mirror** that `checker.rs:1558` pours into
>   `TypeCheckResult`. The naming fact is still single-sourced: candidate-name table (in phase) +
>   span-keyed resolution (cross-stage, `ir_gen.rs:380` constructs, `:7632/:7705` consumes); the
>   definition-side mixed name is self-computed by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassify as in-tree fact** (user ruling A, see the
>   §Obligation Fields table ruling record in this document). What is deleted is the **declaration**
>   in the doc comment "mirrored for middle-side / LSP consumption" (types.rs:47-48); keep the field
>   and its in-phase consumption (`operator_interfaces.rs:623` operator query, seven
>   constraint-solving queries in `inference/expressions.rs`, `checker.rs:214-217` read-only
>   accessor); cross-stage zero reads direct evidence
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire a consumer**: the
>   table's contents are an environment mirror injected by typecheck's pre-table (always non-empty);
>   if LSP really needs it, register under 4.6.x; creating a consumer for a field that "by design
>   doesn't need cross-stage consumption" only introduces fake obligations into the ledger.
>
> After the red list zeroes out, **the field census is closed by "obligations list ∪ non-obligation
> registration (`NON_OBLIGATION_FIELDS`, with reason)"** — gate criterion 1's reverse containment
> holds this end-to-end (add a field forget to register / delete a field forget to sync both turn
> red on the spot). The red-list **mechanism** is preserved: gate criterion 4's staleness check and
> `--selftest`'s `stale-red-list` fixture continue to cover its detection surface; future
> re-registration must go through an explicit ruling.
>
> Synchronized with the clearing: obligation list 16 → **13** (`INVENTORY` / `Obligation::ALL` /
> this document's §Obligation Fields table); gate `--selftest` now covers seven fixture categories
> (newly added "unregistered field", "stale anchor", and "red list empty" three forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Static gate (9.2) actual criteria** (three more hard criteria beyond the one sentence in the
> sketch)
>
> 1. **Three-place reconciliation + reverse containment (F2) + anchor validation (F1)**:
>    `obligations.rs`' `INVENTORY` ↔ `TypeCheckResult` field definitions ↔ this document's
>    "§Obligation Fields" table (including the row-count declaration in the table header), any drift
>    turns red; and the `TypeCheckResult` field set must **equal exactly** "obligations list ∪
>    non-obligation registration (`NON_OBLIGATION_FIELDS`, with reason)" — adding a field without
>    registering, removing a field without syncing, non-obligation registration without reason, two
>    tables overlap → all turn red on the spot;
> 2. **Read-out evidence**: non-red-list obligations must be **really read** (`.field_name` member
>    access; comments/tests/strings don't count) in the **declared consumer file** — this turns
>    "delivery marker ↔ downstream real read" into a hard criterion (the main risk of mechanism A is
>    exactly that the two drift);
> 3. **Delivery marker**: non-red-list obligations must be marked at the delivery point; red-list
>    ones must not;
> 4. **Red-list staleness check**: red-list fields must have zero real reads (a consumer growing
>    turns red), and each must carry a WBS id and an evidence pointer; plus a `--selftest`: ships
>    counter-example / positive-example fixtures, counter-example must be red, positive must be
>    green (proves the detector isn't idling).
>
> **VI. Per-place deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields**: obligation
>    data is still held by `TypeCheckResult`; the ledger only records "list + delivery markers +
>    settlement conclusion". Reason: copying would mean two sources of truth (each field is produced
>    separately by typecheck's checker and the ownership layer), and they will inevitably drift;
>    cross-layer data plane also shouldn't be replicated into the orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, replaced by `is_produced` empty
>    check + `DeliveryLedger` delivery marker. Reason: obligation and product are the same data,
>    `take()` would take away what the downstream needs; and 11 IrGeneration obligations come in the
>    form of "give the whole `&TypeCheckResult` to ir_gen", which can't be taken field by field.
> 3. Sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the conclusion is written into `DriverOutcome.obligations`,
>    the diagnostics split by `ProgramKind`: Check/Lsp go through the per-file collection channel;
>    the rest split by **level** (since 4.3.2 W1090 is E-level) — single-file goes through
>    `PipelineError::TypeCheck` (obligations.rs:654 outbound arm), multi-file/embedded separately
>    record `OrchestratorError` failure (otherwise `result.errors` has no consumer). Reason:
>    returning `Vec<Diagnostic>` loses the form split — Check's diagnostics must be per-file
>    attributed.
> 4. The first-tier criterion of the three-tier table, from "obligation read by declared consumer"
>    to "has delivery marker"; "marker ↔ real read" is left to the static gate (section V criterion
>    2).
> 5. The second-tier of the three-tier table, "escalate to E in `strict` mode" **not implemented via
>    a strict switch**, instead **4.3.2 same-code-escalate-to-E** (no strict switch either, because
>    the healthy-state zero emission is already safe enough, R4).
> 6. Static-gate criteria upgraded from "field appears ≥2 times but no third-file read" to the four
>    in section V.
> 7. The two extra tiers (S1, skip exemption) not in the sketch, see section II.
> 8. S1/S4 table's mentioned single-test name `test_obligations_drained`: actually landed as **20
>    test cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turned green;
>    red-state archived: with the only settlement point removed, `pairs: 0`, 6 failed). The S1/S4
>    two lines have been synchronized to the actual file.
>
> **VII. 4.3.2 landing record (level + gate reinforcement)**
>
> - **Per-field reconciliation input**: `ObligationSettlement.drained_kinds` (the kinds of
>   obligations actually drained this run, available per run) and the five counts (the conservation
>   equation in section II); combined with the audit surface `findings` it can give the fact "has
>   this field ever been consumed by any entry".
> - **Level: same-code-escalate-to-E has landed** (2026-10-13 user ruling A; see section III
>   end-state registration and the D59 D-class breaking item declaration). Implementation:
>   `to_diagnostic` explicitly `.severity(Severity::Error)`; the outbound arm splits the error
>   channel by level. Mutation evidence: remove any delivery marker → that form's compilation fails,
>   `check` exit code is non-zero (`error [W1090]`); after restoration the healthy state emits zero
>   (342/342 corpus zero change). The locale copy has been written in six languages to make clear
>   "W-family code, emission level is error".
> - **Gate reinforcement items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses the `file:line` anchors in **non-obligation
>     registration reason and red-list evidence**, verifying "file locatable (uniquely) + line
>     number registered + the symbol immediately after the anchor really appears in the anchored
>     file"; an anchor pointing to a non-existent symbol turns red. Ships counter-example fixture
>     `tests/baselines/obligations-gate/stale-anchor/` (one for non-existent symbol / out-of-range
>     line each). Note: the red list is currently empty, so the actual landing point of this
>     criterion is the **non-obligation registration** and future evidence pointers (same code
>     path).
>   - **F2/R3 reverse containment**: the gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference", non-empty difference turns red; ships
>     counter-example fixture `unregistered-field/` (newly added struct field not registered).
> - **Information item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable at real
>   entries (a unit whose typecheck failed doesn't keep `type_result`, the obligation doesn't exist
>   in the first place); this tier's semantics is pinned by synthetic-`State` test cases, real
>   end-to-end only covers `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 sub-task closure (red list zeroed + obligation-diagnostic end-state E-level)**
>
> **8.1 Three-round landing timeline** (item by item corresponding to code file:line; this round
> only adds conclusions, doesn't change code)
>
> | Round               | Conclusion                                                                                                                                                                           | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
> | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger online | `State::assert_drained()` only settlement point; 13-entry obligation list + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                       | call `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; outbound `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                                 |
> | 3.4.4 clearing      | `variant_ctor_calls` field deleted — declared duty equivalently fulfilled by in-place shape detection on the IR side                                                                 | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                        |
> | 3.4.5 clearing      | `method_overload_ir_names` cross-stage mirror deleted (kept in-phase candidate-name table + `overload_resolutions`)                                                                  | decision consumer `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                              |
> | 3.4.9 clearing      | `interface_impl_registry` reclassified as **in-tree fact**: delete "mirrored for middle/LSP consumption" declaration, keep in-phase consumption; obligations 14 → 13, red list 1 → 0 | declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment in the same file at `:47-51`, rewritten by ruling A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven sites in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 leveling      | **Same code, escalate to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; outbound splits the error channel by level (first-failure precedence)                | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); the decoupling of code family and level registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                 |
>
> **8.2 Closure conclusion**
>
> - **Red list zeroed**: the number of entries in `INVENTORY` (`src/driver/obligations.rs:201`) with
>   `red_list: Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0).
>   Mechanism **preserved**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's
>   staleness check and `stale-red-list` fixture; the gate outputs "red list 0 entries — already
>   cleared".
> - **Obligation diagnostic end-state E-level**: the **code number and W-family ownership** of
>   `W1090` are unchanged, the emission level is `error` (D5 end state, 2026-10-13 user ruling A).
>   This **belongs to the D-class breaking items of D59** (changes externally observable behavior),
>   already registered in both this note's section III and the RFC-013 "emission channel" section;
>   **healthy state zero emission ⇒ existing corpora and user programs are unaffected** (342/342
>   corpus "diff: empty", IR snapshots zero drift; t11 independent verification and t12 review each
>   re-verified).
> - **Field census closed**: `TypeCheckResult` field set = obligations list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption-surface
>   reason); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No un-landed promises**: all reinforcement items registered in this note have **already
>   landed** (F1/F2/F4/F5, see sections V, VII, VIII); the only two unclosed items are the **known
>   boundaries** in 8.4 and t7 F5 which has been transferred to an issue, neither is an un-landed
>   criterion of this mechanism.
>
> **8.3 Disposition of acceptance / review findings from each round** (marked item by item)
>
> | Source        | Finding                                                                                                                                                           | Status                                                                                                                                                                                                                                                                |
> | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is a plain-text pointer; silently stale when the replaced consumer is removed                                                                   | **Landed**: gate F1 anchor validation (file locatable + line number registered + symbol anchor, `scripts/ci/check-obligations.py:251`); counter-example fixture `tests/baselines/obligations-gate/stale-anchor/`                                                      |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment; silently PASSes when a new field is added without registration                                                        | **Landed**: F2 explicit difference output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; counter-example fixture `unregistered-field/`                                                                                              |
> | t4 R4         | W-only and escalate-to-E caliber not registered                                                                                                                   | **Landed**: this note's sections III / VII + RFC-013 "emission channel" + six-language `locales/*.json` W1090 help and error_output examples                                                                                                                          |
> | t7 F5         | Dead-code analysis's overloaded-method reference count is distorted (legal overload still mis-reported as `W1005`)                                                | **Transferred to issue `#450`**, owned by WBS **4.9 Reference Pool Correct Implementation** (`docs/src/dev/architecture/09-execution-wbs.md:237` registration) — not this sub-task                                                                                    |
> | t11 F7        | Under gate `--root` mode, F1 doesn't cover the anchor promises in module header prose comments                                                                    | **Known boundary** (see 8.4)                                                                                                                                                                                                                                          |
> | t11 F8        | Validator side's self-record: mutation driver once wrote to the main repo by mistake (immediate `git restore` cleared, affected rounds' output voided and re-run) | **Information item** (not a deliverable defect)                                                                                                                                                                                                                       |
> | t11 F9        | Main repo intermediate state (unstaged file made this document's header inconsistent with `INVENTORY`) triggered gate red                                         | **Information item / resolved**: frozen commit self-consistent PASS; "intermediate state committed directly turns the gate red" is exactly F2/F4's design intent                                                                                                      |
> | t12 F12-1     | This document's struct range written as `16-73`, actually `16-76` (no mechanical reconciliation after t9 added 3 comment lines)                                   | **Landed + mechanized**: range corrected to `16-76`, and new gate **F5** added (range anchor, wrong write turns red); same-class numbers aligned by grep in one pass (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                              |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was overwritten by W1090)                                                     | **Fixed** (captain amended commit message to explain): `src/driver/obligations.rs:686`'s `failure.is_none()` guard; this regression was caught by the **existing nail-board** test `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registrations, not un-landed promises)**
>
> - **F1's scan surface**: anchor validation only covers the **text of registered entries** —
>   non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence; **the
>   `file:line` promises in module header prose comments are not in the scan surface** (currently no
>   registered entry is affected). If anchor promises are written in module headers in the future,
>   **manual review must take responsibility**, or F1's scan surface must be expanded — one of the
>   two, not allowed to default to no one in charge.
> - **Static boundary of the gate**: the set-level check of delivery markers is on the static side
>   (criterion 3), the single-arm removal is backstopped by **runtime settlement** — static +
>   runtime double-layer is the designed form, not a gap (t11's M11A/M11B/M11C mutation
>   bidirectional verification).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization
> landing)**: of the 13 obligations above, the **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been converted to **structural-identity keys** (`PlanId`) per RFC-039
> **D20**. This note registers the landing form, criteria, classification basis and known boundary;
> the definition line is still reconciled by gate **F4** (the line numbers in the two rows above are
> the current code facts).
>
> **I. Landing form (carrier takes A: `PlanId`)**
>
> The key is the **structural identity** that follows the AST node, no longer relying on the
> position coincidence of "each side computes the span once": `PlanId` (`src/util/plan_id.rs`, `u32`
> monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` for synthetic nodes outside the
> parser) is issued by the **sole allocator** `ParserState::alloc_plan_id`. Carriers at two places:
> `Stmt.plan_id` (statement identity → release-plan ownership and consumption) and
> `Expr::Call.plan_id` (call-site identity → overload-resolution production and consumption);
> `Expr::call` is the only call-expression constructor in the whole repo, synthetic calls outside
> the parser are all marked `UNPLANNED` (**cannot be consumed**, see section II).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`); the only
>   consumer entry is `ReleasePlan::consume(stmt)` — **read and bookkeeping in the same call**;
>   `ir_gen.rs:360` reads, `:3103` uses (statement identity given by `walk_stmt`'s set/restore + CFG
>   node→statement-identity table; the scope-exit Drop belongs to the last statement in the scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **as one**),
>   production site `inference/expressions.rs:4550` (writes by call-site `PlanId`, the dispatch is
>   destructured transparently from `Expr::Call { plan_id }`), consumer sites `ir_gen.rs:7697` (with
>   binding) / `:7772` (without binding) via `OverloadResolutions::get` (hit means bookkeeping).
>
> **II. Consumer-must-reach settlement (reuses the 4.3 ledger, no second set)**: the consumption
> ledger is the **single implementation** `ConsumptionLedger<K>` (`src/util/consumption.rs`); each
> of the two tables holds one instance; when `generate_module_ir` returns successfully, **the same
> point** marks "one full round run" for both ledgers (`ir_gen.rs:1095` release plan / `:1097`
> overload resolution); the three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), the only settlement point hooks one line each into the two
> obligations — non-empty difference is **E-level W1090** (same code, escalate to E, caliber in
> section III). The exemption criterion hangs on the **ledger's own completion marker** (not on
> whether `unit.ir` exists), so the Check form taking the IR product doesn't constitute an
> exemption.
>
> **III. D59 classification basis (C class, red before green)**: the form
> `Type.method(positional, name = value)` was **silently giving the wrong answer** before the change
> (measured: `r: Int = t.pick(1, a = 2)` compiles cleanly, runs outputting `1` instead of `20` — the
> candidate decided on is dropped, falls back to the last bound in declaration order); after the
> change the same program gets compile-time `error [W1090] ... production key not consumed`. The fix
> takes the **loud** path rather than "stuff the decided name into the named-argument call": the
> latter loses the named arguments, equivalent to another kind of silence (end-state semantics in
> [09](./09-execution-wbs.md) §3.14 / issue #452). Additionally: `Drop` instructions are no-ops in
> the interpreter (`src/backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`), so the key migration of the release plan doesn't change runtime behavior — the
> 342-corpus empty diff is empirical evidence of this.
>
> **IV. Criteria and evidence**: D41 same-form criterion `test_release_plan_spans_consumed` (settled
> by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file lex→parse→typecheck→ir_gen
> on the corpus, **the difference must be empty + evidence floor**: number of files that produced
> resolutions `≥ 1`, number of truly consumed entries `≥ 2`, no whitelist); 9-item decision-form
> coverage nail-board in `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected
> return / positional arg / type-body binding / `?` operand / pure-named / pos+named / ambiguous /
> variant-ctor / unallocated identity). Mutation drive: consumer key shifted by one ⇒ criterion
> red + real program `error [W1090]` + corpus-gate diff; after restoration re-greens. Gate values:
> `check-obligations.py` PASS, corpus 342-entry diff empty, `cargo test --lib` 2712 passed / 0
> failed (IR snapshots zero drift).
>
> **V. Known boundaries (registrations, not un-landed promises)**: the **no-binding consumer site**
> at `ir_gen.rs:7766-7790` is **unreachable** under the current corpus and test surface (method
> declarations uniformly take the with-binding consumer site via `register_type_bindings`), so this
> branch's key correctness **has no red state to falsify**; the reason to keep it = it's the
> symmetric branch of the with-binding path, deleting it would rewrite "method call without
> `type_bindings`" into a silent form. Registered by independent verification (t5 F-1) and review
> (t6 F2), **no new task opened**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — fourth caller of the single-file path
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

**`stages()` can only be the 6 predefined combinations above (one per `ProgramKind`); it does not
accept a freely-passed array from the caller.** This is the key constraint that prevents the
`Program` abstraction from degrading into a "free parameter that accepts anything," asserted by
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
                // … 12 arms, not one can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The only settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: four implementation deviations
> from the sketch above, semantically equivalent, registered for the record —
>
> 1. §4's `StageOutcome` three-state is not returned by the stage arm, but expressed by the `State`
>    failure marker + `Aggregation` gate (Continue / Warn / Abort semantics unchanged, eliminating
>    the per-arm boilerplate return);
> 2. `Driver` has no `config` field — the configuration's only source is `Program.config` (§5 "Avoid
>    Driver holding mutable global state" judgment overrides the sketch field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded into the `DriverOutcome.skipped` internal
>    ledger, not emitted — the S2 "deliberately fix no bug" zero-diff criterion requires; outbound
>    emission follows the 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is the allowed direction); the orchestrator's existing call sites will be
>    discussed again after 4.2 migration is done.
> 5. Under the multi-file form, ProofExecution is an independent stage, running after **all**
>    typecheck (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof in the
>    per-file typecheck loop (file N's proof before file N+1's typecheck). Single failure, the two
>    are byte-for-byte identical; in the "file 1 proof fails + file 2 typecheck fails" multi-failure
>    scenario, the first report changes from proof error to typecheck error (test nail-board).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every found file has an entry, clean file is empty Vec — `check_project` current contract);
>    Discovery product extended to "file set + used_by edge set + shadow events" triple into State
>    (MultiFile form has no downstream consumer, just record don't emit); the check path's per-file
>    parse drops from 2 times to 1 time with the C3 topology.
> 8. Lsp form landing (4.2.3, decided by file-source split, 2026-10-09 user decision): disk-file
>    parse failure is the same as Check's plan B (collect + exit compilation unit — both sides
>    degrade consistently); the edited-buffer parse failure keeps the residual AST and continues
>    typecheck (editor philosophy — typing intermediate state shouldn't make semantic functions
>    disappear). The Discovery arm for Lsp keeps the buffer source overwriting stale disk content;
>    Typecheck under Lsp only checks the target file (other units only feed the registry for
>    signatures); the intra-file diagnostic order normalization principle extends to LSP (typecheck
>    diagnostics → W-code warnings → proof errors). The old behavior where LSP hard-aborted on
>    unrelated disk-file parse errors, and the handler silently fell back to the single-file path,
>    is fixed by this step.
> 9. Embedded form landing (4.2.4): `Program` adds a `shared_registry` field — the embedded std
>    module is a sub-compilation, the registry is **input** not product (EMBEDDED_STAGES has no
>    Registry stage), #94's SymbolTable sharing contract is thus explicitized from "caller remembers
>    to pass the same registry" to a program declaration. Error path text is normalized from
>    `<std.test> (embedded std)` to the unit virtual path `<std/test>` (compiler-internal error
>    surface only, no test pin). At this point all four orchestrator entries are migrated into
>    Driver.
> 10. Standalone check unified (4.2.5, ruling A + IR-stage ruling, 2026-10-09): Check form for a
>     program without project root is a faithful carrier of single-file semantics — the warning
>     surface only covers the entry file (side files only get errors, better to miss than to err);
>     relative `use` resolves along the importer's directory (rustc single-file mod alignment).
>     CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full
>     chain) already runs IR generation, only E3019/E1014/E1015 etc. are produced in ir_gen (runner
>     gate empirical). IrGeneration's Check form is pure check doesn't consume IR; Script/Bin
>     sub-form is expressed by module_key's None/Some (E3023's existing switch, ir_gen.rs:1660).
>     Monomorphization is not in CHECK (only produces resource-exhaustion/internal errors, zero
>     corpus dependency). **〔2026-10-10 empirically withdrawn by 3.4.8〕** — "zero corpus
>     dependency" turns out to be evidence of a blind spot that the corpus has no
>     pathological-recursion fixture; E3005 is the only compile-time line of defense for such
>     programs. See note #14. Side fix of a latent defect: the Registry arm's re-harvesting
>     registration for embedded std units overwrites the native half already merged by with_std()
>     (E1043 false positive) — embedded units now skip duplicate registration.
> 11. LSP single-file fallback unified (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is deleted, replaced by
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI use the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, keeps residual AST to continue
>     typecheck (editor philosophy extended from 4.2.3 ruling; not marking stage failure, otherwise
>     topological skip makes typecheck never run); Typecheck dispatches the
>     `check_module_collect_all` free function. The LSP single-file path thus gains proof (E4018),
>     W-code warnings (W1001–W1005, test nail-board) and IR-level errors; lexical failure changes to
>     report real diagnostics (old synthetic "E0001 lexical error" text disappears with the sequence
>     deletion — real diagnostics carry precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 must-do): the
>     two entries had already converged — the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, the difference carried by
>     whether `collected_errors` is empty; five collection points in statements.rs (function
>     body/use/for/block/while). During verification, a duplicate-diagnostic defect was confirmed
>     and fixed: the collection point puts the first error into both `collected_errors` and the
>     `Err` return channel, pass-3's `add_error` for `Err` causes the same code same span same
>     message ×2 (nested can be ×3); plus two independent mechanisms: annotation-check signatures
>     and overall-annotation double-visit (E1003/E1103), ownership-layer same-point double-emit
>     (E2014/E2018). The fix lands at the module-result boundary: deduplicate by (code, span,
>     message) — diagnostics are positional facts, duplicates carry no information; 63 corpus
>     entries have duplicate copies removed, run column and exit code zero drift (the "duplicates"
>     in `borrow_conflict_err` are artifacts of the baseline triple format without a column number —
>     two E2018s are actually legal diagnostics in different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground`
>     constructing `Program { kind: WasmPlayground }` + Driver — the `Compiler` wrapper layer is no
>     longer in the path. Error text is byte-for-byte aligned to the old `CompileError` Display
>     prefix (Parse error:/Type error:/Internal error:), playground UI zero visible difference.
>     WasmPlayground shares SINGLE_FILE_STAGES + FailFast with SingleFile (4.1 existing declaration,
>     has a real producer since this step).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Empirical refutation of "mono only
>     produces resource-exhaustion noise": pathological generic recursion (`f(x)=f([x])`) without
>     mono on the multi-file path makes the compiler process stack-overflow (0xc00000fd — the
>     interpreter recurses in Rust, no graceful runtime error); mono's depth gate is the only
>     compile-time line of defense; standalone check before 4.2.5 took the pipeline full chain with
>     mono, the exclusion ruling caused a silent coverage regression ("zero corpus dependency" only
>     proves the corpus has no pathological fixture). The "type-erasure fallback" assumption was
>     voided by RFC-033's reflection ruling — `^^List(Int)` needs the real identity of
>     instantiation, monomorphization is the foundation of reflection, not an optional optimization.
>     Four design points: ① typecheck produces instantiation_request for qualified calls
>     (`lib.f(x)`, FieldAccess shape), generic_id uses the qualified name, sharing namespace with
>     merged IR (currently only recognizes bare Var, expressions.rs:2455); ② `Stage::ALL` topology
>     changed to Linking before Monomorphization, MULTI_FILE stage table tail adds Monomorphization
>     — consumes merged_ir, aggregates all units' requests (deferred bucket's containing_fn
>     qualification), mono is upgraded from single-module pass to whole-program pass; single-file
>     stage table unchanged (no Linking, subsequence property preserved, IR snapshots zero drift); ③
>     CHECK adds Linking (pure merge — E3020 entry check stays in RoleClassification, prevent
>     double-report) and Monomorphization (pure check doesn't consume, same as IrGeneration's Check
>     form) two stages; ④ resource protection follows whole-program BFS coverage of cross-unit
>     mutual recursion. Monomorphizer itself is unchanged (consuming ModuleIR + request set contract
>     is exactly the merged-IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Implemented qualified-call instantiation
>     collection per #14 design: `ExpressionInferrer` injects module-qualified key table (`use`
>     three arms registered — normal / alias / per-item alias and only SubModule-kind export),
>     `callee_generic_name` is concatenated via `SymbolTable::qualify` into a qualified name sharing
>     the same source as merged IR; qualified-call first path arity criterion tightened (when
>     argument table can't be resolved, fall back to second path to take arguments by signature,
>     avoiding cross-module same-name function mismatch and mis-reporting E3018). During
>     implementation, three **asymmetric defects** on the mono side were empirically confirmed and
>     fixed in the same round: (1) delete-key (generic-name set) and rewrite-key (containing_fn +
>     name + span triple) granularity mismatch — sites that can't resolve arguments degrade to
>     symbol request into the deferred bucket, when the container function isn't generic the bucket
>     never drains, the original is already deleted by name → dangling call (`list_ops.yx:53`
>     runtime E6006 empirical); fixed by restoring gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the single source
>     of fact to determine coverage, the original of uncovered sites is preserved. (2) The restore
>     segment's `HashSet` iteration order caused the function-table order to be non-deterministic
>     (t2 review F1) → changed to `Vec` and sorted by name, byte-level reproducible product. (3) The
>     non-rewrite-face shapes TailCall / MakeClosure originally looked up the map key the same way
>     (comment claimed conservative) → `call_form_is_rewritten` single-point judgment, non-rewrite
>     shapes uniformly count as uncovered. Each is paired with a nail test (red-state empirical
>     discriminating power before the fix). Corpus diff baseline regenerated one line at the same
>     commit (remove_at frame added `(int64)`). **Commit-strategy note**: when typecheck collection
>     and mono fix were split into two commits, the stash tree containing only the former triggered
>     `stdlib_docs` red on HEAD's existing defect (same family E6006 as `list.is_empty([])`) — the
>     two halves are each other's green-light prerequisite, so merged into one atomic commit.
> 16. 2026-10-10 five-ruling registration (after R1 closure): (1) **D2 ruling A** — user-module
>     exports carry `generic_fn_type_params_snapshot()` along with `Export.type_params`, isomorphic
>     with the std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source
>     for qualified-call monomorphization, signature reverse-inference degraded to fallback (merged
>     into R2 scope). (2) **HKT declared side alive, call side broken** (inferred type arg E1002 /
>     explicit type arg E1010) registered as WBS 3.5 (3 third-tier tasks). (3) **deferred bucket
>     under non-generic container never drains** merged into 3.4.8 ②' s acceptance sub-item: the
>     bucket must drain or fail loudly, no silent non-specialization. (4) **Closure variable
>     name-call runtime E6006** registered as WBS 3.6; semantics (i) support indirect call / (ii)
>     compile-time rejection, **ruling (i) chosen** — indirect call supported: `CallDyn` mechanism
>     is already in place (`ir_gen.rs:7945` closure-expression path), what breaks is `Var` callee
>     being downgraded to a name-based `Call`, and typecheck accepts that program by design. (5)
>     **Trade-off criterion solidified** — a plan's rejection can only be based on correctness and
>     readability; "large change surface / introduces new mechanism / high cost" is not a rejection
>     reason, written into `coding-rules.md` part two and the HOWTO self-check list; trade-offs
>     requiring user ruling follow the `AGENTS.md` "when requesting user ruling (mandatory)" four
>     items (situation+evidence / per-item pros and cons / clear recommendation+cost /
>     breaking-call-out).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     the Linking arm **aggregates the whole-program request before taking each unit's IR** and
>     qualifies `containing_fn` (otherwise the deferred bucket keys and merged IR are not from the
>     same source, the bucket never drains). **① closure (D2 ruling A)**: user-module exports carry
>     the declared-period type-parameter snapshot with `Export::type_params`, isomorphic with the
>     std path (`yx_sources.rs:62/108`). **Defects empirically confirmed and fixed in the same
>     process**: (a) **F1** — `qualify_names` only rewrites function bodies, `ir.init` is never
>     visited; in multi-file, function definitions are already qualified while module-top-level
>     calls are still bare names → runtime E6006 (**reproduces even for non-generic**, not related
>     to generics); fixed by having init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) mono side "whole-program unique suffix" heuristic with missed silent
>     `continue` → delete the heuristic, missed now **loud E3018** (ambiguous silent mismatch is
>     worse than not parsing). **Regression caught and fixed by independent verification**: t6 found
>     that named import `use lib.{pick}` followed by bare-name call was falsely rejected by E3018
>     (IR instruction name is alias-qualified name, request name is bare name, aggregation side only
>     qualifies by unit key) → fixed by aggregation side **directly calling ir_gen's**
>     `import_aliases_from` and `resolve_call_target` (extracted to a pub single point, one place
>     produces, one place consumes), and matrix nail tests cover two forms of named import.
>     **Filed**: init-segment `DefId` blank empirically is "consistency blank" not a live defect
>     (codegen name fallback can resolve, four-class init calls empirically pass), registered as WBS
>     3.7.4 to close the "function body goes through def, init relies on name" caliber fork.
> 18. init-segment DefId caliber fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure), function
>     body and `ir.init` share the same implementation and the same `resolve` closure; 208 IR
>     snapshots regenerated only 309 pairs of `(def=_)`→`(def=dN)` changes (other-lines 0; whole
>     tree init segment 0 residual `(def=_)`). codegen's DefId main path and name fallback two
>     states produce translation result and execution value item by item consistent (four-class init
>     calls measured), corpus baseline zero drift; probe flips and strengthens per new caliber
>     (identity + exact index + execution value, fallback init coverage turns red). **Errata
>     (2026-10-11)**: the above commit numbers were rewritten by `git pull --rebase` — the original
>     b554497b/e60dcc87/b7ac7396 correspond in order to the post-rebase 375789d2/d922a551/38bd4b9e;
>     the pushed dev history takes the new numbers (the old numbers were local temporary numbers,
>     not in the remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④' s design basis in #14; ②' s
>     landing in #17, this entry is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` does only pure merge
>     (aggregate request + `link_module_irs`), the entry check is wholly left to
>     `RoleClassification`; `ir_generation_check` produces and keeps `us.ir` (memory only); mono
>     failure in Check goes to the collection channel, in MultiFile keeps `fail_with`;
>     `into_outcome` first `take_check_diagnostics` then judges failure (structural failure no
>     longer clears already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + t5 cross-form matrix) and one review (t6) all pass; corpus diff
>     340/340 line-by-line IDENTICAL, IR snapshots zero drift, test count 2643→2656. **Existing gaps
>     found and fixed in the same round**: F1 (rootless multi-file check's request name and IR
>     definition key from different sources → silent non-specialization, see WBS 3.4.8 row's landing
>     note) — fix is aggregation side exact reverse-lookup by real definition key (non-heuristic),
>     first-version fix is aggregation side exact reverse-lookup by real definition key
>     (`flat_defs`); **same-day erratum**: the bridge was deleted in the regression fix; root cause
>     instead solved by `module_key` dual-duty decoupling (`script_mode`), the three share the same
>     source `{unit key}.{name}`. The regression (CI doc-example gate 258 blocks 10 fail → 0 fail)
>     is located and fixed per WBS 3.4.8 row's landing note. **Filed (existing, not this round's
>     regression)**: E3005 help text doesn't match resource-protection semantics; multi-file E3005's
>     span label is offset (HEAD binary reproducible); request-collection face's type-parameter
>     position-dependence (same face, back to 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic-surface closure (2026-10-12, 809b9b936 +
>     8605a081b + subsequent two commits). **3.9.1 (executor-layer guard reachable)**: interpretive
>     calls changed to **explicit stack-driven** —
>     `CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push frame then return control to
>     the outer `step_one`, frame carries `ReturnTarget{Host|CallerSlot|Discard}`, on return write
>     back to the caller's slot, multiple-call instructions continue via `pending_calls` within the
>     frame. Baseline measured each layer of interpretive call consumes native stack **2544 B**,
>     crash threshold at `count_down(394)` (Windows main thread 1 MiB); after the fix, 20 000 layers
>     of recursion can run on a 512 KiB stack thread (about 0 per layer). The guard (`push_frame`)
>     is thus reachable at any entry, any `max_stack_depth` value; `ExecutorError::StackOverflow` is
>     changed to carry the real limit (the original `util/diagnostic/mod.rs` always passed 0). **The
>     original 3.11.2 (deep-stack backtrace truncation) per user ruling is merged into this task**
>     (same file, can't be split by file): backtrace changed to first 16 frames + count ellipsis +
>     last 8 frames, shallow stack unchanged frame-by-frame. **3.10.1 (compile-time ungated native
>     recursion, #447)**: locate first then change — crash point is `MonoType::type_name` recursion
>     (**328 B/layer**, SP probe + per-op critical-point determined), same path also has derived
>     `Clone` (~400 B/layer), `substitute` internal deep-copy, `type_name_hash` (164 B/layer),
>     `collect_generic_type_refs`; after changing four places to work-list and removing each layer's
>     one-time whole-request deep-copy, pathological generic recursion with `mono.max_depth=5000`
>     produces **E3005 (with real limit)**, no more stack overflow. **3.11.1 (task-boundary
>     error-code fidelity, #448)**: task-result error side changed from value to **structured**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`),
>     cross-boundary restores `ExecutorError` by value; task-internal E6001/E6003/E6004/E6008 in
>     Standard mode and Embedded message are verbatim identical. Two **pub API payload-shape
>     changes** are marked `BREAKING CHANGE` in the commit. **Commit-strategy note**: 3.9.1 and
>     3.11.1 **are not file-splittable** — `interpreter/executor/executor.rs` simultaneously carries
>     the trampoline and the error-channel's production/consumption sites, `TaskFailure` is defined
>     in `runtime/engine.rs`; stashing only one side fails to compile (pre-commit compiled with
>     **stash subset**, twice rejected empirically), so per #15's same caliber, merged into **one
>     atomic commit**. **Known boundary and follow-up**: Drop derive (~75 B/layer) ⇒
>     `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate is carried on
>     native recursion (~3 696 B/layer, default 256 layers ≈ 924 KiB), currently zero call site,
>     production unreachable, future wiring must close in the same family; Standard-mode task
>     interpreter constructed with default `ExecutorConfig` ⇒ its E6004 limit is always 1024 (config
>     propagation registered as WBS 3.12, user ruling B: separate team); test runtime budget
>     separately opened as issue #449 (pathological fixture 76s, clippy dual target, commit runs
>     full), not part of this RFC's construction. **Corpus directory layering (same round,
>     8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures split
>     into 23 category-based sub-directories, 88 IR snapshots migrated in mirror path; corpus
>     baseline regenerated at the same commit (342 lines, independently recomputed line-by-line
>     comparison: only `file` field and 2 new changes), 41 snapshots regenerated due to longer
>     file-header paths, line-by-line check **only span offset changes** (after offset-normalization
>     zero difference).
> 21. LSP semantic-data pipeline unified (4.6.1, 2026-10-16). Diagnostics and semantic data produced
>     by **the same compilation**, in-project cross-file references carry `resolves_to` pointing to
>     the real definition — the old form had didOpen/didChange trigger **two trips** of compilation:
>     diagnostics go through the unified Driver pipeline, semantic data goes through server.rs's
>     manual lex→parse→single-file typecheck (no module registry, in-project `use` resolution fails,
>     reference events not produced, cross-file jump/rename/reference lookup silently fails — the
>     D59-registered silent failure is closed in this task). **Implementation**: (1) `Export`
>     carries `definition_span` (extract_module_info harvests from AST identifier node, same source
>     same value as typecheck definition event's `def_span`; native std always None), `ModuleInfo`
>     carries `definition_file` (Registry arm fills in by DiscoveredFile); (2) when checker drains
>     reference events, via `import_def_locations` redirect cross-file imports' `resolves_to` from
>     the use line to the real position in the definition file (std import has no registration,
>     keeps falling to the use line); (3) checker module identity changed from unified `"<module>"`
>     to unit file path (this value is only consumed on the semantic-data surface, zero diagnostic
>     consumption — empirically `env.module_name` all 4 consumption sites); (4) `DriverOutcome` adds
>     `semantic_db` channel — CollectAll form (Lsp target file regardless of diagnostic success +
>     SingleFile fallback) aggregated in, Lsp arm tail injects non-target-file top-level
>     export-definition surface from registry (`external_definitions`, unopened files can also jump;
>     type/sub-module exports not injected — typecheck-side type registration doesn't produce
>     definition events, both sides keep consistent gap); (5) orchestrator adds
>     `analyze_source_in_project` (`check_source_in_project` transferred to call and only takes the
>     diagnostic half); (6) LSP-side `analysis.rs` single-compilation entry replaces the manual
>     pipeline (server.rs's `update_semantic_db` deleted), World key unified to normalized file URI
>     (`uri.rs`: `path_to_uri(uri_to_path(u))` idempotent normalization, drive-letter case and %XX
>     encoding differences normalized to the same key). **Side fix (D59 B-class branch — old
>     behavior silently does nothing, doesn't change any originally-successful program's external
>     appearance)**: in-project files' cross-file jump/rename/reference lookup changed from silently
>     failing to hitting the real definition; lexical error time, old manual pipeline deleted
>     semantic info, changed to residual AST staining preserved (4.2.6 editor philosophy extension);
>     when the target file has a type error, semantic data no longer discards along with
>     `type_result` (staining/jump doesn't disappear in typing intermediate state). **Nail board**:
>     6 driver-level (target file path key / cross-file resolves_to + external-definition face
>     round-trip / std import fallback to use line / error file preserved / single-file fallback
>     path key + error preserved), 1 LSP e2e (red-then-green empirical — before fix, jump returns
>     None), 4 URI normalization.

**Ten entries' refactoring (function by function)**:

| Entry                                                      | After refactoring                                                                                                                                                                                                          |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to constructing `Program { kind: SingleFile, … }` and handing to Driver                                                                                                                       |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                    |
| `compile_project` (`orchestrator.rs:99-237`)               | Slimmed to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                              |
| `check_project` (`orchestrator.rs:273-399`)                | Slimmed to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                                |
| `check_source_in_project` (`orchestrator.rs:450`)          | Changed to a single Driver call + filter results to target file                                                                                                                                                            |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Changed to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                                 |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (calls `run_project` or `SingleFile`)                                                                                                                                                                            |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (calls Driver)                                                                                                                                                                                                   |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changes to `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence for the single-file branch deleted**, changed to calling `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, uniformly takes `Program { kind: Check }`                                                                                                         |

**The degree of duplication between `check_project` and `compile_project` (about 40% shared / 60%
diverged)**. The shared parts: vendor-consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parsing loop, per-file checker assembly, result handling. The diverged parts
are items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **That 40% shared skeleton is exactly the
value zone of `Driver`** — the diverged 60% are all "different stage sets" or "different aggregation
modes", which is exactly what two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issue, continue
}
```

**Key design decision: "upstream failure causing this stage to skip" is not a stage return value.**

Reason: skipping is decided by **topology**, not by the stage itself. If we let each stage return
`Skipped`, then the "why didn't I run" information would be scattered across 12 stages and cannot be
centrally audited. Instead, `dispatch` decides at the start of the loop based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
is fixing — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip" that
produces no diagnostic, simply because the obligation happens to be empty. With the rule
established, any "because X wasn't done, so Y didn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text direction                                                                                                                                 |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` caused it                              | "Stage Y was not executed because upstream stage X failed"                                                                                     |
| Predecessor obligation empty caused it                  | "Stage Y was not executed because there was no obligation to process (normal)" — **this item's severity = Info, not counted in warning count** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y was not executed because configuration was not enabled"                                                                               |

The second class is key: it makes "normal skip" distinguishable from "abnormal skip" in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` relies on that count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts immediately (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current state correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)              | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early-exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project files and single-file files in the same process, while CLI's `run` and `check` are two
separate calls; putting them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of
"early-exit or not" — they are two different checker entries. After unification, it should be driven
by the `Aggregation` parameter on the same implementation, not by keeping two functions (**this is
an additional item this document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representations, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                                            |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what migrates into the ledger is **settlement
responsibility**, not the type's owning module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the rest of the field types stay in
place; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3
consumers (e.g. `ir_gen` reads `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`** — this is consistent with the
driver-no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not
reverse-`use crate::driver`**. `Driver` appearing in `TypeChecker`'s imports is considered a
violation, intercepted by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before refactoring                       | After refactoring                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` for standalone no longer takes this pipeline — uniformly changed to `compile_project` (MULTI_FILE stage table), Script semantics driven by rootless; this row now only describes the `lib::run_file` / `Compiler::compile` library paths) |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already stemmed in P3; **warning surface stays unexpanded** (decision B1, 2026-10-09: dead-code family warnings don't go into the compile path — pool semantics is being corrected by 4.9, normalizing warning surface waits for after 4.9; this row's draft "new warning output" is void)                          |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already stemmed in P3                                                                                                                                                                                                                                                                                                               |
| LSP (in-project)                    | No proof_execution                       | Already stemmed in P3                                                                                                                                                                                                                                                                                                               |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                          |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                              |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This fits the RFC-027 §8 positioning of "not bound to a specific solver", and is also
the contract already declared by `backend.rs:60` ("caller should degrade conservatively") — the
current implementation is the opposite of its own contract document.

### Compiler Change List

**New files (6)**

| File                        | Contents                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file / N for multi-file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line range                         | Change                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                             |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                    |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead-code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to take Driver                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or changed to be driven by `Aggregation` parameter                                                                                                                                                                                                              |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changed to calling Driver                                                                                                                                                                                                                                                                      |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` uniformly takes `Program { kind: Check }`                                                                                                                                                                                                                                                                          |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                   |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently goes through `Compiler::compile_with_source` implicitly landing on path 1)                                                                                                                                                                         |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct the comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/layers/README.md`     | Whole text                         | Layer-order table changed to the **actual** execution order; remove "lower layer fails, upper layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access; the three consumer sites in `predicate.rs`/`checker.rs`/`ownership.rs` change to take the shared access. `&'static` bare-reference form is infeasible (`dyn Solver` is not `Sync`); original text follows D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse sequence
at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct — **the problem is on the consumer side, not the producer side**, changing
the producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (wasm branch reachability determination is filed under
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                      | Compatibility                                                              | Disposition                                                                                                                                               |
| ----------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                        | **Breaking**: previously-silently-passed constraints now report E4018      | `test_multifile_proof_obligation_not_dropped` **written red first** then fixed; release in batches, expected diagnostic-set change goes through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                      | **Breaking**: previously-clean-compiling programs start producing warnings | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                                     |
| First run of `assert_drained()` produces W-level diagnostic | **Breaking**: diagnostic set adds                                          | W first then E, S2/S4 two steps                                                                                                                           |
| `predicate.rs` panic changed to diagnostic                  | **Improvement**: no longer crashes                                         | No break                                                                                                                                                  |
| `build` / `dump_bytecode` subcommands                       | **No impact**                                                              | These two paths don't go through proof_execution                                                                                                          |
| `TypeCheckResult` field migration to `Obligations`          | **Internal refactoring**                                                   | If there's external dependency on `pub` API, sync needed; repo-internal consumers are all listed in the change list                                       |

---

## Implementation Notes

This document corresponds to RFC-039 global stage sequence's **P3 (fix correctness
vulnerabilities)** and **P4 (stage contract and unified Driver)**. The following S1–S5 are the
implementation order within this document, **each step's acceptance criterion references the
[Equivalence Criteria document](07-equivalence-oracle.md)'s C2 category** (orchestration change:
same **diagnostic set** at each entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot skip.** The vulnerability criterion must be written as failing first.

| Deliverable                                                                                                                                              | Acceptance                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                            | **Must be red**. If accidentally green, that means this document's vulnerability analysis needs to be re-verified |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48 — not mixed into single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of single-file and multi-file two-version corpora are comparable                              |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                           | Mark `#[ignore]`, turn green at S4 (red-state archived: with the only settlement point removed, `pairs: 0`)       |

**Rollback point**: no code change, pure new tests.

### S2: Introduce `Stage` + `Program`, Without Changing Behavior

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path's diagnostic set and exit code are byte-for-byte identical**                 |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately fixes no bug** — it only moves existing behavior into the Driver. The
acceptance standard is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restored to original.

### S3: Merge Orchestrator Four Entries + Fix Vulnerability

| Deliverable                                                                                                                     | Acceptance (C2)                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four entries' diagnostic sets are identical after normalizing by `Aggregation`  |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project            |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                      |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                             |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` produces a diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics without Z3 environment                                         |

**Rollback point**: the vulnerability fix and structural merge are **two commits**. If the
structural merge has issues, only the structural commit can be rolled back, keeping the
vulnerability-fix commit — at this time `test_multifile_proof_obligation_not_dropped` stays green;
the reverse (keep structure, roll back fix) turns red, which is an unacceptable intermediate state,
prohibited from merging.

### S4: Enable Obligations Ledger

| Deliverable                                  | Acceptance                                                                                                                                 |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 test cases turn green (implementation form in §2 revision note; the test count is based on that file) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                                   |
| Obligation diagnostic W → E escalation       | Full corpus diff followed by manual review of every new E                                                                                  |

**Rollback point**: `assert_drained()`'s severity level can be controlled by config item; W/E
switching doesn't need code structure changes.

### S5: Proof Layer and wasm Wrap-Up

| Deliverable                                                                                                                                                                                                             | Acceptance                                                                                                                                                         |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                  | Documentation corresponds item-by-item with `checker.rs` call sites                                                                                                |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown not in cache** — otherwise one timeout is固化 across compilations via process-level cache, and pollutes cargo test inter-thread sharing) | Production-path `default_solver()` call site zeroed (only tests keep); cache hit rate observable across three consumer sites (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                         | Diagnostic when no `body_checker`                                                                                                                                  |
| `orchestrator.rs` 12 wasm attributes reachability determination                                                                                                                                                         | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination need                                                                  |

**Note**: correcting the layer order will change the diagnostic set, possibly exposing a lot of
previously-silent `Unproven`. **This item should go independently of S1–S4**, not mixed in the same
PR as the entry merge.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **rejecting runtime registration**                                                                                                                                                                                                                                                                                                                                                         | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism proven effective by the opcode table in RFC-039 routing table B. `StageScope` further turns "per-module or project-level" into a type fact, eliminating call-count questions that need human reasoning                                                                                             |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, paired with the **static** gate of `scripts/ci/check-obligations.py`; **end-state severity E-level** (2026-10-13 user ruling A: same code escalate to E, code number unchanged; **implementation form in §2 revision note**: the ledger doesn't copy the field structure; red list — after clearing 0 entries — mechanism preserved, doesn't enter user-visible diagnostic stream) | What we fix is an entire class of bugs, not one bug: at least 2 hidden dangers of the same kind have been verified currently (`proof_calls`, `checker.rs:1313`), all 13 obligation fields are in range. W→E in two steps is to make every step's C2 criterion usable — now the obligation list has been cleared to 13 entries, the red list is zero, healthy state emits zero, so the end state directly takes E (see §2 revision note sections VII, VIII) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **don't touch** `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branches' producer-side logic                                                                                                                                                                                                                                                                                         | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic is fixed", and the logic of these three branches itself is correct                                                                                                                                                                                                                                                    |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment promises" and "code behavior"**. The `checker.rs:5165`'s "must not
  let silent pass be revived" is a **comment-level contract**, fulfilled by `assert_drained()`.
- **"One array forgotten to append"**. Currently five functions each hand-write the call sequence;
  after unification it's the only stage table, and missing a new stage's hook will error at compile
  time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — of the 11 inconsistencies, 7
  (two implementations of dead-code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type interface**, but are configuration questions of "whether
  to run some analysis"; it also can't express `Aggregation`, it can only move the either-or of
  `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, which is
  equivalent to trading "five functions each hand-writing the call" for "one array forgotten to
  append", which is exactly the cause of the current 11 inconsistencies. This repo already has a
  same-form counter-example: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` /
  `HotReloader` described by `docs/src/dev/design/check/` are all zero-implemented.
- **Only patch bugs, don't move structure** — can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` is still scattered in 5 functions, the next fork point will keep
  growing from here. **It must be done first** (is part of S1/S3), because structural refactoring
  needs a known-red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`** — `check_module` is a general entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` can't hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is looking for the bug at the wrong
  layer: the bug is in the orchestration layer, the detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. The 139-line
  `compile_project` split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactoring.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent problems"**. After the fix, there may be a batch of "new error" user reports —
  they are real bugs, just never reported. Must be clearly stated in CHANGELOG.
- **`assert_drained()`'s severity switching needs per-field manual judgment**. For some obligation
  fields (e.g. `module_namespaces`), "no consumer" may be by-design needing no consumption,
  shouldn't warn. S4 needs to go through field by field, can't be one-size-fits-all.
- **`Program` abstraction may be premature**. If some entries' stage sets are unstable for a long
  time, `stages()` will degrade to "passing a different array each call" free parameter, contract
  constraints become void. **Mitigation: `test_program_stage_coverage` asserts that `stages()` can
  only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability-fix criteria depend on the
  [Equivalence Criteria document](07-equivalence-oracle.md)'s vulnerability tests going red first.
  **If that document doesn't first establish the multi-file corpus layer, S1 cannot be accepted** —
  because the existing 293 corpora all take the single-file path, with zero coverage for this class
  of defect. This document doesn't cover the IR verifier (`verify_loose`); that prerequisite work is
  not in this document's scope.
- **Layer-order correction will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, the actual order of `termination` and `ownership` is opposite to the declared).
  This is a diagnostic-set-change risk, should go independently.
- **Thread safety after SMT backend changed to singleton is undetermined**. `backend.rs:13-19` has
  already noted that `Solver` is only `Send`, not `Sync`; `Z3Backend`'s cache is `RefCell`; after
  changing to a cross-compilation-unit shared singleton, need to confirm that all access paths go
  through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependence defects** (e.g. the actual order-dependence of
  termination and ownership). Should be opened after the equivalence criteria are stable.

> **The open questions originally listed in this section have all been ruled on.** The decisions are
> item-by-item in [RFC-039 Decision Log](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

- [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction conventions,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`' `include!` refactoring (construction steps filed under
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration "must not let silent
  pass be revived"
- `src/frontend/core/typecheck/layers/README.md` — Layer-order call table and the real identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT-initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 clearing)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The playground's fourth compilation caller (single-file path, so executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — `cdylib` + `wasm-bindgen` are in the shim crate, not the main crate
- `build.rs:19-55` — Error-code build-time gate, the example of this project's enforcement mechanism
