# Compilation Stage Contract and Obligation Ledger

> **Subsidiary design document.** This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, the grading of acceptance criteria, and the execution stage order are in the
> RFC-039 main body; the positioning of each subsidiary document is in
> [this directory's index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction drawing for RFC-039. It addresses one
question: **"Which stages ran in this compilation, which didn't, and why"** is currently scattered
across six paths and ten entry functions with manual wiring, and no single place can answer this
question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture rather than logic.**

The comment in `src/frontend/core/typecheck/checker.rs:5165` literally says "Unproven → compile
error, no degradation, no silent pass... do not let silent pass return." But this invariant only
holds on the **single-file path**: proof-function constraints like `x: Sorted(3)` silently pass
under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP — the proof function
is never executed, and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, not registered at runtime) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry-function transformation of the ten
  entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency direction specification, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 main body

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactor and **the order**; this document gives L1's **concrete
shape**, **per-file change list**, and **this document's internal implementation stages**
(corresponding to P3 "fix correctness vulnerabilities" and P4 "stage contract and unified Driver" in
RFC-039's global sequence). Where this document conflicts with RFC-039, RFC-039 takes precedence.

Equivalence criteria are executed per the **C2 (orchestration change)** category of the
[Equivalence Criteria Document](07-equivalence-oracle.md): same diagnostic set per entry + same
corpus behavior.

## Current State

> **All items in this section are verified facts**, each with file path + line numbers. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Only "Write It Wrong Once, the Whole Repo Stays Silent" Structural Defect

It differs in nature from the other two categories of defects (module boundaries, test wiring):

| Defect                  | Typical Manifestation               | Is There a Signal?       |
| ----------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntax error    | Source code is wrong                | Has diagnostic           |
| Type mismatch           | Type is wrong                       | Has diagnostic           |
| Stage wiring missing    | Some stage is never called          | **No signal whatsoever** |
| Obligation not consumed | Field filled in but no one reads it | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what wasn't written**. This is exactly the root cause diagnosed in RFC-039: "This
project treats 'design' as a documentation convention, not as an executable constraint."

### Existing Enforcement Mechanisms in This Repo

The same repo already has mature enforcement mechanisms — they just haven't been extended to the
stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are **build-time
  hard-gated** by `build.rs:19-55` via `tools/code-tables`, comparing each one against the RFC-013
  code table; any inconsistency directly `panic!`s and refuses to compile.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` directory has 6227 lines, accounting
  for 47.9%) has high test density, and each module's test subtree is wired up declaratively — this
  is the most completely test-wired block in the entire repo and can serve as a formal reference for
  stage-layer gates.

**Design capability is sufficient. What's missing is "putting a same-level gate at the orchestration
layer too."**

(Line count basis: `(Get-Content).Count`, see the "Line count basis" section in
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                         | Which Compilation                                                                  |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Direct call of 5 stages                                                            |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + link                                                |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collect all diagnostics                                        |
| 3c  | **LSP in project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embed std**              | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | In-project goes to 3c, otherwise manual lex→parse                                  |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                       |
| ------------------- | ------------- | -------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                        |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded in it** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below provides evidence cell by cell. **A blank does not mean "this entry doesn't do this
thing" but rather "no code in this entry does this thing"** — this is the essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                    | **No**                                                      | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                    | Yes `356-377` (role-aware, no config gating)                | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                           | Yes `350`                                                   | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, drops `used_by`/`shadow_events`)       | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (independent ModuleIR then merge) |

> **Review note (WBS 3.4.3, verified cell-by-cell on 2026-10-07)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, preserved unchanged. Differences in current state after P3
> lands:
>
> - **Row 1 fixed**: proof_execution is shared by all five entries via the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the single-consumer defect
>   no longer exists.
> - **Rows 2/4/5 unchanged** (multi-file `run` still doesn't emit W1003 or run the dead code family)
>   → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization exclusive to single file) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3, potential risk unproven defect).
> - **Row 7** check_module / check_module_collect_all dual entry → assigned to 4.2.7
>   (parameter-driven by Aggregation).
> - The "Ruling #434" referenced on the code side was previously zero-registered in docs — now
>   retroactively registered as RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new out-of-table fact, registered 2026-10-09)**: The
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, hard switch in #117); the multi-file `discover` previously only recognized
>   explicit `use` — in-project for-loops compiled cleanly but caused runtime E6006 (empirically
>   verified by probe). Fixed and WBS 4.10.2 struck off; the same-family "multi-file missing one
>   step of single-file" parse hard-abort quirk registered as WBS 4.10.1 — **also fixed** (landed
>   immediately after 4.2.2, Check path downgrades to per-file collection; this audit method
>   examines the "stage coverage / field consumption" dimension, this item is a compilation-unit
>   membership difference, an escaped dimension).
>
> - **Embedded std registry surface coverage (discovered during 4.2.5 construction, fixed)**: The
>   Registry arm re-harvests registrations for embedded std units via `extract_module_info`,
>   overwriting the native half-face already registered by `with_std()`'s "native + yx surface
>   merge" (`result.is_err` etc. lost → std.test falsely reports E1043). This audit's third escaped
>   dimension: "two registration sources for the same module key" merge semantics — the single-file
>   path's registry is only formed once via `with_std()`, but the multi-file/Check Registry arm
>   inserts per-unit to get coverage.

**Two locations need precise expression, otherwise they will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` under the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; under the **single-file**
  path (`run_file` → `Pipeline::run`) it does report. The cause is in `compile_project` — it pushes
  the full `TypeCheckResult` (including `warnings`) into `type_results`, but **this function has
  zero `result.warnings` read sites throughout**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The entry `main` criterion for row 11 is also from a different source**: `check_project:385-395`
  uses `surfaces.bins` (the manifest-declared surface); `compile_project` uses `is_bin_role`
  (`250-252`, only checks "whether there is a manifest"). The two functions give different answers
  to "which files must define `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. The following eight steps are all verifiable.**

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

The semantics are: "the predicate arguments of this refinement constraint are compile-time literals,
and the proof function must actually be executed to determine this."

**Step 2 — The consumption point is inside checker, but only produces, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The literal text of `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass. ` ...
> `This branch therefore becomes a true defense line, not a forward-looking bottom: do not let silent pass be resurrected.`

That is: the author knew "producing no diagnostic" was a defect, explicitly categorized it as a
**known, nowhere-to-go intermediate state**, and assumed by design that "someone will definitely
come and read `proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it, `checker.rs:1439` writes into `TypeCheckResult` with
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code has **only one read site: `src/frontend/pipeline.rs:187`**
(passes as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the whole repo fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` is the production-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four entries of orchestrator skip that layer entirely.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), `compile_embedded_module` (`1374`) **all do not call
`pipeline.rs`**, calling `TypeChecker::check_module` directly (`124` / `493` / `1386`). So they
don't even reach that single read point at `pipeline.rs:187`.

**Step 6 — Consequence: even the embedded std module's own refinement obligations go through the
discard path.** `compile_embedded_module` (`1374`, the `check_module` call is at `1386`) is
responsible for compiling the embedded std. This means **the proof obligations of the embedded std
module itself are also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) under the multi-file / check / LSP three paths **compiles
cleanly, runs cleanly, with no diagnostic whatsoever**. The proof function was never called.

**Step 8 (supplementary verification) — There is a second silent discard point within the layer.**
`checker.rs:1303-1314` handling ownership layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and
no ledger entry.** This directly contradicts the "do not let silent pass be resurrected" promise
from the `5164` branch, and is **independent of the orchestrator issue** — even if the entry layer
is completely fixed, this one will remain silent. (Handling timing: should be processed in the same
batch as the obligation ledger, because it is the same kind of problem as the obligation ledger.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits for the keywords `Sorted`
/ `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example `rfc027_return_refinement.rs:350-359` asserts a `SumUpTo` call exists in
`result.proof_calls`. But they all **go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping precisely
before `pipeline.rs`**.

The test file's own documentation comment already makes this clear.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are in .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation was filled", while the
bug is "the consumer doesn't read it".** A test that only tests the producer side and not the
consumer side is naturally immune to this class of defects.

### A Stronger Finding: The Equivalence Criteria's Main Corpus Is the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) uses end-to-end diffing of 293 `.yx`
corpora in `tests/yaoxiang/` as the **third-tier criterion** and the primary acceptance means for C2
stages. But empirically:

- There is **no `yaoxiang.toml` at all under the `tests/` directory** (zero hits in whole-directory
  glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  and none cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is
empirical evidence of this point. That file is annotated with `// expect: compile-error E4018` (line
15), and its header comment on line 7 says "Status: ❌ Should be rejected at compile time" — **it
can pass precisely because it goes through the only path that actually executes the proof
function**.

**Conclusion: The third-tier criterion needs to supplement a multi-file corpus layer, otherwise it
cannot serve as this document's acceptance tool.** See implementation point S1.

### Obligation Fields: 13 "Produced Equals Contract" Fields, Zero Mechanism Guarantee

In `src/frontend/core/typecheck/types.rs:16-76`'s `TypeCheckResult`, there are **13 fields** whose
doc comments explicitly say "produced by stage X → consumed by stage Y", i.e. they are **cross-stage
obligations** by nature:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-taking convention**: Definition line = the **declaration line** of that field in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of the doc comment — the comment block for the
> same field may span multiple lines, and the declaration line is the only anchorable position. The
> **struct range** mentioned in the sentence above this table
> (`src/frontend/core/typecheck/types.rs:16-76`) floats with the struct's line numbers and is
> mechanically reconciled by gate **F5** (write it wrong and it goes red); the definition line
> column is reconciled by **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | |
> `release_plan` | `38` | ownership → IR generation (**`PlanId`-keyed**: `ir_gen.rs:360` reads
> (`type_result.release_plan.clone()`), `:3103` uses (`ReleasePlan::consume`); settlement-complete
> marker `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | | `instantiation_requests`
> | `42` | typecheck → monomorphization (`pipeline.rs:381`) | | `existential_coercions` | `44` |
> typecheck → IR generation | | `implementation_proofs` | `46` | typecheck → IR generation | |
> `sum_types` | `54` | typecheck → IR generation | | `sum_type_param_names` | `57` | typecheck → IR
> generation | | `operator_dispatches` | `59` | typecheck → IR generation (span-keyed) | |
> `overload_resolutions` | `62` | typecheck → IR generation (**`PlanId`-keyed** — call-site
> identity: produced at `inference/expressions.rs:4550`, consumed at `ir_gen.rs:7697` / `:7772`
> (`OverloadResolutions::get`); settlement-complete marker `:1097`) | | `try_expr_impls` | `65` |
> typecheck → IR generation (span-keyed) | | `match_scrutinee_types` | `69` | typecheck → IR
> generation (span-keyed) | | `module_namespaces` | `72` | typecheck → IR generation |

**Ruling record (3.4.9, 2026-10-13 user ruling A)**: The original row 14 `interface_impl_registry`
(`types.rs:49`) has been **reclassified as an in-tree fact** — by design it is consumed by typecheck
during phases and has zero cross-stage reads, not an obligation:

- **At phase consumption points (living)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), `inference/expressions.rs` seven constraint-solver query sites
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), `checker.rs:214-217`
  read-only accessor, `checker.rs:974` injects body_checker.
- **Zero cross-stage reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; the `src/driver` side only has its
  own ledger registration and docs (already removed this round).
- **Disposition**: delete the **declaration** of "mirrored for middle-end / LSP consumption" in the
  doc comments, keep the field and in-phase consumption; this field is registered in
  `src/driver/obligations.rs`'s `NON_OBLIGATION_FIELDS` (with reason). If LSP really needs this
  table in the future, it goes through a new 4.6.x registration, **no consumer is manufactured in
  this round**.

**All of these fields are passed across layers keyed by `Span` or name tables.** The Equivalence
Criteria Document has already pointed out that span-keyed contract mismatches "silently fail, with
no error whatsoever" (`ReleasePlan` is typical: any inconsistency in how both sides compute span →
`Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the consumer
  is in a downstream module, and the downstream module is on the must-traverse path of all entries.
- The consumer of `proof_calls` is at the **top of `pipeline.rs`**, belonging to **another entry's
  implementation**. The four orchestrator entries don't traverse the `pipeline.rs` layer at all.

**Verified fact**: currently the entire repository has **no mechanism whatsoever** to ensure these
13 fields are consumed. The only "protection" is that `ReleasePlan` happened to ride along with IR
generation.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Each field in `TypeCheckResult` that is not a cross-stage
obligation is registered one by one (field name + reason), **complementary** to the "§Obligation
Fields" table above:

| Field                     | Why It's Not a Cross-Stage Obligation (Key Points)                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `module_name`             | Unit identity metadata (0 hits in `src/middle`)                                                                                                  |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produced equals contract")                                                                      |
| `bindings`                | Has cross-stage reads (ir_gen.rs:735/:5034), but doc comment doesn't declare a cross-stage contract ⇒ not collected                              |
| `local_var_types`         | Used for diagnostic rendering (ir_gen.rs:852/:4543/:4794), same as `bindings` — no declared contract                                             |
| `semantic_db`             | Consumer is in LSP (from 4.6.1 emitted through `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`), not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads ir_gen.rs:4336/:6867, doc comment doesn't declare contract                                                                     |
| `warnings`                | Warning diagnostic channel (same kind as `diagnostics`)                                                                                          |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; see "§Obligation Fields" ruling record for in-phase consumption points)                                       |

**Why not "wire up a consumer"**: manufacturing a downstream for these fields would only introduce
false obligations, dressing up "by design no cross-stage consumption is needed" as a contract
defect; the correct closure mechanism is **reverse containment** — the `TypeCheckResult` field set =
obligation list ∪ non-obligation registration (hard-enforced by `scripts/ci/check-obligations.py`
criterion 1: add a field and forget to register, delete a field and forget to sync, non-obligation
registration missing a reason, two tables overlap — all go red on the spot).

### Four Additional Contract Defects in the Proof Layer

The following all come from the proof layer analysis, independent of the entry-fork problem, but all
belong to the category of "stage contract not enforced".

**(a) Layer order is only declared in the docs, with no mechanism to enforce it; the `equivalence`
layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (positioning is anchored by function name / driver point, line
numbers taken from the current tree):

| Actual Order | Call Site                                                                                                                                          | Corresponding Layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement walk driver `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                    | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                       | `ownership.rs`      |
| 4            | Type constraint solving `env.solver().solve()` (not in `layers/`)                                                                                  | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                             | `equivalence.rs`    |

Layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) only exist in
each module's own header comment, and don't correspond to the call order; the README has no "number
→ call timing" mapping.

`equivalence` is only used by `inference/assignment.rs:17` as an `is_subtype` utility function,
unrelated to `ProofResult`. At the same time **no short-circuit exists at all** —
`check_module_impl` `add_error`s termination's errors one by one and continues to walk down, "lower
layer fails, upper layer doesn't run" is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-fail panic has been eliminated by 3.3.1 in P3 (SOLVER slot
> made `Option`, missing case conservatively downgrades to `SMTResult::Unknown`); "silently skipped
> (not injected)" supplemented with W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is executed per **RFC-039 D58**, real fix location `proof/smt/backend.rs` (new
> process-level shared singleton + `with_shared_solver` closure port), the
> `default_solver() → &'static` expression in the 02 change list is governed by D58 (the `&'static`
> raw-reference form is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                | Acquisition Strategy                                     | Solver Unavailable                                        |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Construction-time injection `with_solver_owned`          | `None` → **silently skipped** (not injected)              |
| `ownership.rs:627` (back-edge cut decision)             | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively don't cut |

The comment in `predicate.rs:31-32` explicitly chooses hard failure: "Initialization failure
maintains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds
kernel capability'." While the contract document at `backend.rs:60` says "`None`: backend
unavailable... **the caller should conservatively degrade**". **Two philosophies coexist in the same
code, and the hard-failure one panics rather than returning an error.**

**(c) Each back-edge decision creates a new Z3 context, the cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge cut decision. While
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — each call does `Z3Backend::new()`, i.e. creates a
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, but the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call mode**.

(Comparison: `predicate.rs:34-36`'s `LazyLock` is genuinely a singleton. Same backend, two lifetime
strategies.)

**(d) Checker silently degrades + comment doesn't match reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call ownership table
```

When `body_checker` is `None`, the ownership check gets **an empty type ledger and an empty call
table** and runs as usual — ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least
leave a trace in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving". Actually `self.env.solver().solve()` is at `checker.rs:1320` — **termination is
at `1256` and ownership is at `1296` after, i.e. what the comment calls "before" is actually "two
layers after"**.

### wasm Current State: shim crate hosts, 29-file branch is alive

The wasm target **is built and in CI**, with `cdylib` not in the main crate but in an independent
shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate only has `rlib`                        | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has wasm target dependency section     | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                   |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, downgrades to warning if missing)                                                                                                                                                            |
| **29 files** contain the `wasm32` literal         | Of which **22** have actual `#[cfg(...)]` attributes, another 7 only mention it in docs (for the list and per-item judgment see the "F2 Judgment" subsection of `06-cleanup-inventory.md`)                                                                                        |
| `orchestrator.rs` 16 occurrences                  | 10 attributes (4 positive + 6 negative) + 6 comment occurrences                                                                                                                                                                                                                   |
| `src/lib.rs` 11 occurrences                       | All attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                        |

**Conclusion: these 29 files' `#[cfg(target_arch = "wasm32")]` branches are load-bearing.** They
determine which APIs in the main crate can be called by `wasm/src/lib.rs` (152 lines) under the wasm
target — `src/lib.rs:151/170/187` gate `run_file` / `run_project` / `build_bytecode` entirely (these
three need `std::fs`), while the shim takes a different path.

**But this brings a fact directly related to this document**: the wasm playground entry
`compile_playground` (`:84`) in `wasm/src/lib.rs` constructs `Program { kind: WasmPlayground }` and
hands it to `Driver::run` (JS-side entry `run_code`, `:122`) — **single-file path**. Therefore:

| Paths that consume `proof_calls`                          | Whether they execute proof functions |
| --------------------------------------------------------- | ------------------------------------ |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                              |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                              |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                              |
| Multi-file `run` (`compile_project`)                      | **No**                               |
| `check` (`check_project`)                                 | **No**                               |
| LSP in-project (`check_source_in_project`)                | **No**                               |

That is: **the existence of `wasm/` changes "proof_execution has only one consumer" to "has
three"**, but all three are on the single-file path side. The unified Driver has this path already
covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing the same single-file
stage table with `SingleFile`, `:135`); landing record in 4.2.8 row of `09-execution-wbs.md`
(`:272`).

(Cleanup scope — which of the 10 wasm attributes (4 positive + 6 negative) in `orchestrator.rs` are
unreachable in the playground scenario — is assigned to the "F2 Judgment" subsection of
`06-cleanup-inventory.md`: 4 wasm arms are runtime-unreachable in playground but compile-time
load-bearing, 6 `not(wasm32)` arms are load-bearing across the three native stage tables, no
directly-deletable low-risk items. The open question in RFC-039 "build a wasm target, or delete
these branches" has **already expired**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Registered at Runtime)

**Core constraint: `Stage` is a compile-time exhaustible enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time "new stages must be handled
by the orchestration layer" — this is the same mechanism the opcode table has already proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexical + syntax               PerModule
    Registry,           // Module registry construction            Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type check (including embedded proof layer)   PerModule
    DeadCodeAnalysis,   // Dead code family analysis              Project
    ProofExecution,     // Proof function compile-time execution         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module link / IR merge       Project
}

impl Stage {
    /// All stages. When adding new variants, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs once
at the project level) make "which stages must run per module, which must run once at the project
level" a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once in
`dispatch` — this eliminates the need for human reasoning in questions like "should
`allocate_global_slots` be called for each file" currently in `orchestrator.rs`.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the draft are corrected by
> implementation evidence —
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR output
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), the draft order contradicts the data
>    flow.
> 2. The Check variant's stage table is normalized to dead code **before** proof by `Stage::ALL`
>    topological order: in `check_project`'s current state, proof runs before dead code, the two
>    have no data dependency, diagnostic set is the same (C2 set semantics), only in-file diagnostic
>    order is normalized. The single-file path (dead code embedded in typecheck, before proof)
>    already conforms to ALL order, byte-for-byte acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST output — the
>    draft order would force the Registry arm to "hide parse", making the stage table lie about the
>    data flow; and multi-file parse would drop from 2 times to 1. Current-state faithful
>    registration as a side note: multi-file path parse errors used to be hard-aborted (the `?`
>    propagation in `build_registry_from`, tension with CollectAll semantics registered as WBS
>    4.10.1) — **4.10.1 is fixed** (2026-10-09 user ruling: rust-style collection semantics + plan
>    B): Check path parse failure downgrades to per-file diagnostic collection, ill files exit the
>    compilation unit (don't enter registry, importers report E5001); MultiFile path keeps
>    hard-abort (correct semantics under FailFast where a bad file can't produce IR, long-term
>    nailed down).
> 4. Check variant landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data
>    dependency edges 14→16: `RoleClassification` consumes Discovery's used_by edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadow events — the draft missed these (these two
>    products were produced by `discover_with_used` in the orchestrator era, dropped by `discover`
>    wrapping, lost the data flow by not entering the table); b. In-file diagnostic order
>    normalization extended to the complete form of note #2 above: E3020 entry check moved into the
>    RoleClassification arm (check has no Linking stage, the arm holding surfaces and AST is
>    responsible for validation), in-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); diagnostic set unchanged (C2 set semantics), only
>    stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no one consumes" from undetectable into a fact that can be
asserted at compile time or test time. RFC-039 has already listed this as "the single most valuable
item".

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
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                               | Disposition                                                                       |
| ------------------------------------ | ------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Obligation read by declared consumer | Field in `Obligations` is `take()`-en / marked consumed | Pass                                                                              |
| Obligation non-empty but no consumer | Field non-empty and not consumed                        | **Produce diagnostic** (`W` level, default; `strict` mode escalates to `E` level) |
| Obligation empty                     | Field is empty                                          | Pass (no need to consume)                                                         |

**Why start with W level, not E level**: fixing obligation discarding will **change the diagnostic
set**. C2 criterion requires "same diagnostic set per entry"; if escalated to E in one go,
`yaoxiang check` will suddenly produce large amounts of previously-silent `Unproven` diagnostics.
Split into two steps (W observation first, then E) makes every step's criterion usable. See
implementation point S4 for details.

**`assert_drained()` has a single call site**: `Driver::run` at the end of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third file reads it → fail). `assert_drained()` is the **runtime** gate, the script is
the **static** gate, the two are complementary and both needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed)**: This note registers the
> **implementation form, measured numbers, and 4.3.2 interface** of the sketch above and the
> three-tier table, corresponding file:line by file:line. Landing commit `1941cc47a` (verified tree
> `99079bc52`). It is also the **third of three** "no-silent" bearing points on the red list (the
> other two: audit surface, static gate).
>
> **One, deliverables (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (`:201`, field name /
>   consumer stage / red list), **non-obligation field registration `NON_OBLIGATION_FIELDS` (`:150`,
>   8 entries with reasons)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), emission `emit_obligation_diagnostics` (`:654`), non-consumer
>   gating `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped` (`:549`),
>   IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Single settlement point**: at the end of the `Driver::run` stage table, before
>   `into_outcome()` produces `CompilationResult` — `src/driver/mod.rs:110`
>   (`State::assert_drained`, obligations.rs:570). The `Aggregation::FailFast` early-exit path is
>   also settled (the ledger must balance).
> - **Seven delivery points** (one per arm; remove any one and runtime W1090 will definitely fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only external read path of
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — same delivery port
>   at all four locations: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single-file direct read of
>   request table) / `:1221` (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295`
>   reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   conservation formula
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()`
>   nailed by use cases.
> - Diagnostic code: W1090 registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, reason written in entries above `:26-31` — settlement granularity is "unit
>   × obligation", no user-addressable source location, no dummy span stuffed); six-language copy in
>   `locales/*.json`. **Emission level**: from 4.3.2 onward it is **error** (same code escalated to
>   E, see paragraph three) — determined by explicit `.severity(Severity::Error)` in `to_diagnostic`
>   (obligations.rs:444), code number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, with `--selftest`).
>
> **Two, three-tier table cell-by-cell implementation + two added tiers**
>
> | Three-tier table situation           | Implementation criterion                                                                             | Code                   |
> | ------------------------------------ | ---------------------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by declared consumer | **Delivery marker exists** (marker ↔ real read is reconciled by the static gate, see paragraph five) | obligations.rs:618-623 |
> | Obligation non-empty but no consumer | Non-empty + consumer stage ran + no marker → W1090 (**error level**, see paragraph three)            | obligations.rs:624-630 |
> | Obligation empty                     | `Obligation::is_produced` checks each field for empty → pass                                         | obligations.rs:587-591 |
>
> Two **added tiers** (2026-10-13 user ruling, not in the sketch):
>
> 1. **S1**: The obligation's consumer stage is not in the stage table of this `ProgramKind` → **not
>    judged as owed** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: Consumer stage is skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not judged as owed (obligations.rs:598-602). `NoObligations` does **not**
>    exempt — when an obligation is non-empty and the stage claims no obligations, that exactly
>    means the ledger is unbalanced, must report.
>
> **Three, emission caliber (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: this mechanism only emits **W level** (W1090); **no strict
> switch**. **4.3.2 (final state, 2026-10-13 user ruling A)**: **same code escalated to E** —
> `W1090` code number unchanged, emission level raised to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code** (same fact
> with two codes equals parallel representations). **Escalation only changes the emission level, not
> the emission conditions.** **This is a D-class breaking item of D59 (changes externally observable
> behavior), registered faithfully**: a healthy program emits zero (when all obligations have
> delivery markers, no report), so existing corpus and user programs have zero impact (342/342
> "diff: empty"); but once a run actually hits an undelivered obligation, that diagnostic **counts
> toward `error_count`, renders `error[W1090]`, exit code is non-zero** — unlike the previous "W
> level doesn't block compilation". The caliber of decoupling code family from emission level has
> been registered in RFC-013 "Emission Channel" section (code-number prefix only determines
> numbering attribution, level is determined by the explicit severity at the construction point).
> Mechanism fact (measured): severity takes the explicit `.severity()`, otherwise "code starts with
> `W` → Warning" (`codes/builder.rs:186-203`), the registry's `ErrorCategory` **does not
> participate** in severity; exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`), **W1090 walks
> `error_count` (from 4.3.2) because of explicit error**.
>
> **Red-list fields don't enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C;
> current red list is zero, see paragraph four), the reason is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is filled from the
>   typecheck preset table, **constantly non-empty**, what's emitted per compilation is a **constant
>   signal** (no distinguishability), but it would break three existing external contracts — "clean
>   program zero diagnostic / LSP zero diagnostic / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing use cases turn red**, of which **8 are
>   on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; another 4 in
>   `src/driver/tests/`), corpus baseline 24/342 fixture changes, and the only viable fix is "filter
>   W1090 out of zero-diagnostic assertions" = relax existing criteria.
> - Ruling B (downgrade to Info level) would still break 9/12 of the list-style assertions, and Info
>   is not "output by ruling level".
> - Ruling C measured: existing use cases **zero change**, corpus baseline **zero change** (342/342
>   "diff: empty, behaviorally equivalent"), runtime W1090 becomes a **discriminating regression
>   detector** — healthy tree silent throughout, fires only when a delivery marker is removed (three
>   delivery points each had marker-removed mutations, CLI `check` printed 2 `warning [W1090]` when
>   markers were removed from the Check arm).
>
> The red list's "not silent" is borne by three points: ① `DriverOutcome.obligations.findings` audit
> surface (carried every run, with WBS id and evidence pointer); ② `check-obligations.py` prints the
> red list (currently empty) + staleness check every run; ③ this note.
>
> **Four, red list current state (cleared) and landing records of 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0** — the three clearing tasks all landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen generates
>   CreateVariant") is fulfilled equivalently by IR-side in-place form detection:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST form
>   `FieldAccess{Call(Var)/Var}` + taking declaration order from `sum_type_variants`), call site
>   `:7496` replaces the entire Call with `CreateVariant`; the function's doc explicitly says "span
>   key across systems is unreliable (both sides have different semantics for Call node span), so
>   detect in-place" — **wiring up a consumer would re-introduce the already-fixed span mismatch
>   defect**. Deletion surface: `types.rs` field, `checker.rs:1489-1493/:1565` collection,
>   collection chain in `inference/expressions.rs` and `inference/statements.rs`, `VariantCtorCall`
>   struct in `environment.rs`; `infer_variant_ctor_call`'s type inference and all diagnostics
>   **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (keep in-phase candidate name
>   table and `overload_resolutions`). All read/write points: written in-phase
>   (`environment.rs:205-219` registered in same order as `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550` resolution point, unique hit writes to `overload_resolutions` at
>   `:4550`); what's deleted is the **zero-reader mirror** of `checker.rs:1558` poured into
>   `TypeCheckResult`. Naming fact still expressed in a single point: candidate name table
>   (in-phase) + span-keyed resolution (cross-stage, `ir_gen.rs:380` construct, `:7632/:7705`
>   consume); mixed names on the definition side are self-computed by `method_def_ordinals`
>   (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as in-tree fact** (user ruling A, see the
>   ruling record in §Obligation Fields Table of this document). What's deleted is the
>   **declaration** of "mirrored for middle-end/LSP consumption" in the doc comment
>   (`types.rs:47-48`), keep the field and its in-phase consumption (`operator_interfaces.rs:623`
>   operator query, `inference/expressions.rs` seven constraint-solver query sites,
>   `checker.rs:214-217` read-only accessor); cross-stage zero reads direct evidence
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the content of this table is an environment mirror filled by the typecheck preset table
>   (constantly non-empty), if LSP really needs it, it goes through 4.6.x new registration;
>   manufacturing a consumer for a field that "by design doesn't need cross-stage consumption" only
>   introduces false obligations into the ledger.
>
> After the red list goes to zero, **the field census is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reason)"** — gate criterion 1's reverse
> containment protects this end-to-end (add a field and forget to register / delete a field and
> forget to sync both go red on the spot). The red list **mechanism** is kept: gate criterion 4's
> staleness check and `--selftest`'s `stale-red-list` fixture continue to cover its detection
> surface, future re-registration must go through explicit ruling.
>
> Caliber synchronized with the clearing: obligation list 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / §Obligation Fields Table of this document); gate `--selftest` now covers seven
> categories of fixtures (newly added "unregistered field", "stale anchor" and "red list is empty"
> three forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **Five, static gate (9.2) actual criteria** (three more hard criteria than that one sentence in
> the sketch)
>
> 1. **Three-place reconciliation + reverse containment (F2) + anchor validation (F1)**:
>    `obligations.rs`'s `INVENTORY` ↔ `TypeCheckResult` field definitions ↔ this document's
>    "§Obligation Fields" table (including the row-count declaration in the table header), any drift
>    goes red; and `TypeCheckResult`'s field set must **equal exactly** "obligation list ∪
>    non-obligation registration (`NON_OBLIGATION_FIELDS`, with reason)" — add a field and forget to
>    register, delete a field and forget to sync, non-obligation registration missing a reason, two
>    tables overlap, all go red on the spot;
> 2. **Read-out evidence**: non-red-list obligations must be **really read** in the **declared
>    consumer file** (`.field_name` member access; comments / tests / strings don't count) — this
>    turns "delivery marker ↔ downstream real read" into a hard criterion (mechanism A's main risk
>    is exactly the two drifting);
> 3. **Delivery marker**: non-red-list obligations must be marked at delivery points; red-list ones
>    must not be marked;
> 4. **Red-list staleness check**: red-list fields must have zero real reads (growing a consumer
>    goes red), and each entry carries a WBS id and evidence pointer; also `--selftest`: comes with
>    negative/positive fixtures, negatives must go red, positives must go green (proving the
>    detector isn't idling).
>
> **Six, point-by-point deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …field… }` **does not copy field by field**: the
>    obligation data is still held by `TypeCheckResult`, the ledger only records "list + delivery
>    marker + settlement conclusion". Reason: copying creates two sources of fact (each field is
>    produced separately by typecheck's checker and ownership layer), inevitably drifting; the
>    cross-layer data surface also shouldn't be copied to the orchestration layer.
> 2. The sketch's `take()` / "mark consumed" was **not adopted**, replaced with `is_produced` empty
>    check + `DeliveryLedger` delivery marker. Reason: the obligation and the product are the same
>    data, `take()` takes away what the downstream needs; and the 11 IrGeneration obligations have
>    the form of "the entire `&TypeCheckResult` is handed to ir_gen", which can't be taken field by
>    field.
> 3. Sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the conclusion is written to `DriverOutcome.obligations`,
>    diagnostics are split by `ProgramKind`: Check/Lsp go through per-file collection channel; other
>    forms split by **level** (from 4.3.2, W1090 is E level) — single-file goes through
>    `PipelineError::TypeCheck` (obligations.rs:654 emission arm), multi-file/embedded additionally
>    record `OrchestratorError` failure (otherwise `result.errors` has no consumer). Reason:
>    returning `Vec<Diagnostic>` loses form splitting — check's diagnostics must be attributed per
>    file.
> 4. The first tier of the three-tier table's criterion fell from "obligation read by declared
>    consumer" to "has delivery marker"; "marker ↔ real read" is handed to the static gate
>    (paragraph five, criterion 2).
> 5. The second tier of the three-tier table's "`strict` mode escalates to E" **didn't go through a
>    strict switch**, replaced with **4.3.2 same code escalated to E** (escalated even without a
>    strict switch, because healthy state zero emission is safe enough, R4).
> 6. Static gate criterion upgraded from "field appears ≥2 times but no third file reads it" to
>    paragraph five's four.
> 7. The two added tiers (S1, skip exemption) not in the sketch are in paragraph two.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table: actually landed
>    as **20 use cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turn
>    green; red state left on file: after removing the single settlement point `pairs: 0`, 6
>    failed). The S1/S4 two rows have been synchronized to the actual file.
>
> **Seven, 4.3.2 landing record (level + gate hardening)**
>
> - **Per-field review input**: `ObligationSettlement.drained_kinds` (the obligation kinds actually
>   drained in this run, available per run) and five counts (paragraph two conservation formula);
>   combined with the audit surface `findings` you can give the fact "whether this field is consumed
>   by any entry".
> - **Level: same code escalated to E has landed** (2026-10-13 user ruling A; see paragraph three's
>   final state registration and D59 D-class breaking item declaration). Implementation surface:
>   `to_diagnostic` explicit `.severity(Severity::Error)`; emission arm splits error channel by
>   level. Mutation evidence: remove any one delivery marker → that form's compilation fails,
>   `check` exit code non-zero (`error [W1090]`); after restore, healthy state zero emission (corpus
>   342/342 zero change). The locale copy has written in six languages "W family code, emission
>   level is error".
> - **Gate hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses **non-obligation registration reasons and
>     red-list evidence**'s `file:line` anchors, validating "file is locatable (unique) + line
>     number is in the table + the symbol immediately after the anchor really appears in the
>     anchored file"; if the anchor points to a non-existent symbol, it goes red. Comes with
>     negative fixture `tests/baselines/obligations-gate/stale-anchor/` (symbol doesn't exist / line
>     number out of range, one each). Note: the red list is currently empty, so the criterion's
>     actual landing point is the **non-obligation registration** and future evidence pointers (same
>     code path).
>   - **F2/R3 reverse containment**: the gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = diff", non-empty diff goes red; comes with
>     negative fixture `unregistered-field/` (struct adds a new field not registered).
> - **Information item F3**: `SkipReason::UpstreamFailed` exemption is unreachable at real entry
>   points (units where typecheck failed don't keep `type_result`, the obligation doesn't exist in
>   the first place); this tier's semantics is nailed by a synthetic `State` use case, real
>   end-to-end only covers `ConfigDisabled` (t3 T06).
>
> **Eight, 4.3 sub-task closure (red list zeroed + obligation diagnostic final state E level)**
>
> **8.1 Three-round landing timeline** (each corresponds to code file:line; this round only
> retroactively logs conclusions, doesn't change code)
>
> | Round               | Conclusion                                                                                                                                                                          | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
> | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger online | `State::assert_drained()` single settlement point; 13-entry obligation list + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                    | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                  |
> | 3.4.4 Clearing      | `variant_ctor_calls` delete field — declared responsibility fulfilled equivalently by IR-side in-place form detection                                                               | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                              |
> | 3.4.5 Clearing      | `method_overload_ir_names` delete cross-stage mirror (keep in-phase candidate name table + `overload_resolutions`)                                                                  | Resolution consumption point `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                         |
> | 3.4.9 Clearing      | `interface_impl_registry` reclassified as **in-tree fact**: delete "mirrored for middle/LSP consumption" declaration, keep in-phase consumption; obligation 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is in same file `:47-51`, ruling A rewrite); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, `src/frontend/core/typecheck/inference/expressions.rs` seven sites (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor `:214-217`); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level setting | **Same code escalated to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; emission splits error channel by level (first failure priority)                     | `src/driver/obligations.rs:450` (severity), `:676-699` (E level branch); the registration of decoupling code family from level is in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                 |
>
> **8.2 Closure conclusion**
>
> - **Red list zeroed**: in `INVENTORY` (`src/driver/obligations.rs:201`), the number of entries
>   with `red_list: Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0).
>   Mechanism **kept**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's
>   staleness check and `stale-red-list` fixture; the gate output is "red list 0 entries — cleared".
> - **Obligation diagnostic final state E level**: `W1090`'s **code number and W family attribution
>   unchanged**, emission level is `error` (D5 final state, 2026-10-13 user ruling A). This **is a
>   D-class breaking item of D59** (changes externally observable behavior), registered in both
>   paragraph three of this note and RFC-013 "Emission Channel" section; **healthy state zero
>   emission ⇒ existing corpus and user programs zero impact** (corpus 342/342 "diff: empty", IR
>   snapshot zero drift; t11 independent verification and t12 re-review each re-verified).
> - **Field census closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption surface
>   reason); gate F2 explicitly outputs the diff (currently `21 - 13 - 8 = 0`).
> - **No un-landed commitments left**: all hardening items registered in this note have **landed**
>   (F1/F2/F4/F5, see paragraphs five, seven, eight); the only two not closed are 8.4's **known
>   boundary** and the t7 F5 that was turned into an issue, neither is an un-landed criterion of
>   this mechanism.
>
> **8.3 Disposition status of each round's acceptance / re-review findings** (item by item)
>
> | Source        | Finding                                                                                                                                                             | Status                                                                                                                                                                                                                                                                     |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red list evidence is plain text pointer, will silently go stale when alternative consumers are removed                                                              | **Landed**: gate F1 anchor validation (file locatable + line number in table + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                    |
> | t3 F2 / t4 R3 | Gate criterion 1 missing reverse containment, silently PASSes when new field is unregistered                                                                        | **Landed**: F2 diff explicit output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                                |
> | t4 R4         | W-only and escalate-to-E calibers not registered                                                                                                                    | **Landed**: this note's paragraphs three / seven + RFC-013 "Emission Channel" + six-language `locales/*.json`'s W1090 help and error_output samples                                                                                                                        |
> | t7 F5         | Dead code analysis has distorted reference counts for overloaded method calls (legal overloads still falsely report `W1005`)                                        | **Turned into issue `#450`**, assigned to WBS **4.9 Reference Pool Correct Implementation** (`docs/src/dev/architecture/09-execution-wbs.md:237` registered) — not this sub-task                                                                                           |
> | t11 F7        | In gate `--root` mode, F1 doesn't cover anchor commitments in module header prose comments                                                                          | **Known boundary** (see 8.4)                                                                                                                                                                                                                                               |
> | t11 F8        | Validator tool side self-record: mutation driver once wrote to the main repo by mistake (immediate `git restore` cleared, affected rounds' output voided and rerun) | **Information item** (not a deliverable defect)                                                                                                                                                                                                                            |
> | t11 F9        | Intermediate state in main repo (unstaged files made this document's table header and `INVENTORY` out of sync) triggered gate red                                   | **Information item / resolved**: frozen commit self-consistent PASS; "intermediate state direct commit goes red gate" is exactly the design intent of F2/F4                                                                                                                |
> | t12 F12-1     | This document writes struct range as `16-73`, actually `16-76` (t9 added 3 lines of comments without mechanical reconciliation)                                     | **Landed + mechanized**: range corrected to `16-76`, and new gate **F5** (range anchor, wrong write goes red); similar numbers aligned by grep together (`17 → 20 use cases`, `16 span-keyed fields` → 13)                                                                 |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was covered by W1090)                                                           | **Fixed** (captain has amended commit message to explain): `src/driver/obligations.rs:686`'s `failure.is_none()` guard; this regression was caught by **existing nailed-down** use case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registration items, not un-landed commitments)**
>
> - **F1's scan surface**: anchor validation only covers **registered entries'** text —
>   non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` commitments in module header prose comments are not in the scan surface** (no
>   registered entries currently affected). If anchor commitments are written in module headers in
>   the future, they need to be **borne by human review**, or extend F1's scan surface — take one of
>   the two, don't allow no-one-managing by default.
> - **Gate's static boundary**: delivery marker's set-level validation is on the static side
>   (criterion 3), single-arm removal is **backstopped by runtime settlement** — static + runtime
>   double-layer is the design form, not a gap (t11's M11A/M11B/M11C mutation bidirectional
>   verification).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization landed)**:
> Among the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) per RFC-039
> **D20**. This note registers the landing form, criteria, classification basis, and known
> boundaries; the definition lines are still reconciled by gate **F4** (the line numbers in the two
> rows of the table above are the current code fact).
>
> **One, landing form (carrier takes A: `PlanId`)**
>
> The key is a **structural identity** that follows the AST node, no longer relying on the position
> coincidence of "both sides compute span separately": `PlanId` (`src/util/plan_id.rs`, `u32`
> monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes
> outside the parser) is issued by the **sole allocator** `ParserState::alloc_plan_id`. Carrier at
> two locations: `Stmt.plan_id` (statement identity → release plan's ownership and consumption) and
> `Expr::Call.plan_id` (call-site identity → overload resolution's production and consumption);
> `Expr::call` is the only call expression constructor in the whole repo, synthetic calls outside
> the parser are uniformly recorded as `UNPLANNED` (**cannot be consumed**, see paragraph two).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the sole
>   consumer entry `ReleasePlan::consume(stmt)` — **read and ledger the same call**; `ir_gen.rs:360`
>   reads, `:3103` uses (statement identity given by `walk_stmt`'s push/pop + CFG node→statement
>   identity table; Drop ownership of scope exit goes to the last statement of that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in-one**),
>   production point `inference/expressions.rs:4550` (written by call-site `PlanId`, dispatch from
>   `Expr::Call { plan_id }` destructure pass-through), consumption point `ir_gen.rs:7697` (with
>   binding) / `:7772` (without binding) via `OverloadResolutions::get` (hit-and-ledger).
>
> **Two, consumption-must-reach settlement (reuses 4.3 ledger, doesn't build a second set)**: the
> consumption ledger is a **single implementation** `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), each table holds one instance; when `generate_module_ir` returns
> successfully, it **marks both ledgers'** "completed one full round" at the same point
> (`ir_gen.rs:1095` release plan / `:1097` overload resolution); three-state mapping is unique
> (`consumption_state` in `src/driver/obligations.rs`), the single settlement point connects each of
> the two obligations with one line — non-empty diff means **E-level W1090** (same code escalated to
> E, see paragraph three for caliber). The exemption criterion is hooked on the **ledger's own
> completion marker** (not on whether `unit.ir` exists), so check form taking the IR product doesn't
> constitute an exemption.
>
> **Three, D59 classification basis (class C, red-first-then-green)**: The form
> `Type.method(positional, name = value)` was **silently giving the wrong answer** before the change
> (measured: `r: Int = t.pick(1, a = 2)` compiled cleanly, runtime output `1` instead of `20` — the
> resolved candidate was discarded, fell back to the last binding in definition order); after the
> change, the same program has compile-time `error [W1090] ... production key not consumed`. The fix
> takes **loud** rather than "stuffing the resolution name into a call with named arguments": the
> latter would drop named arguments, equivalent to another kind of silence (final semantics see
> [09](./09-execution-wbs.md) §3.14 / issue #452). Also: `Drop` instructions are no-op in the
> interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only does
> `advance()`), so the release plan's key migration doesn't change runtime behavior — the 342-corpus
> diff being empty is empirical evidence of this.
>
> **Four, criteria and evidence**: D41 same-type criterion `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and new `test_overload_resolutions_consumed` (per-file corpus
> lex→parse→typecheck→ir_gen, **diff must be empty + evidence surface floor**: files with produced
> resolution `≥ 1`, real consumed entries `≥ 2`, no whitelist); 9-item nailed-down resolution form
> coverage in `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return /
> positional / binding in type body / `?` operand / pure named / positional+named / ambiguous /
> variant constructor / unassigned identity). Mutation driver: consumer key shifted by one ⇒
> criterion red + real program `error [W1090]` + corpus gate diff; after restore, turns green. Gate
> values: `check-obligations.py` PASS, corpus 342 diffs empty, `cargo test --lib` 2712 passed / 0
> failed (IR snapshot zero drift).
>
> **Five, known boundaries (registration items, not un-landed commitments)**: The **unbinding
> consumer point** at `ir_gen.rs:7766-7790` is **unreachable** in the current corpus and test
> surface (method declarations uniformly go through `register_type_bindings` to take the binding
> consumer point), so this branch's keying correctness **has no red state to falsify**; the reason
> to keep = it's the symmetric branch of the binding path, deleting it would change "no
> `type_bindings` method call" form into silence. Registered by independent verification (t5 F-1)
> and re-review (t6 F2), **no new task**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embed std)
    WasmPlayground,    // wasm/src/lib.rs's compile_playground — the fourth caller of single-file path
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

**`stages()` can only be the 6 predefined combinations above (one per `ProgramKind`), does not
accept callers passing arrays freely.** This is a key constraint preventing the `Program`
abstraction from degenerating into "free parameter that accepts anything", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides whether to skip, not decided by the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // ... 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Single settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for reference —
>
> 1. §4's `StageOutcome` three-state is not returned by the stage arm, but expressed by `State`'s
>    failure marker + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saving
>    per-arm boilerplate returns);
> 2. `Driver` has no `config` field — the sole source of configuration is `Program.config` (§5
>    "Avoid Driver holding mutable global state" judgment covers the sketch field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded in the `DriverOutcome.skipped` internal
>    ledger, not emitted — S2 "deliberately not fix any bug" zero-diff criterion requires this;
>    emission comes with the 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is an allowed direction); the orchestrator's existing call sites will be
>    revisited after 4.2 migration.
> 5. In the multi-file form, ProofExecution is an independent stage, running **after all** typecheck
>    (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof with the per-file
>    typecheck loop (file N's proof before file N+1's typecheck). For single failures, both are
>    byte-for-byte identical; in the "file 1 proof failure + file 2 typecheck failure" multi-failure
>    scenario, the first report changes from proof error to typecheck error (test nailed down).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have empty Vec — `check_project` current
>    contract); Discovery output extended to "file set + used_by edge set + shadow events" triple
>    into State (MultiFile form has no downstream consumer, just recorded not emitted); Check path's
>    per-file parse drops from 2 to 1 with C3 topology.
> 8. Lsp form landing (4.2.3, split-by-file-source ruling, 2026-10-09 user decision): disk file
>    parse failure same as Check's plan B (collect + exit compilation unit — both sides degrade
>    converge); edited buffer parse failure keeps incomplete AST and continues typecheck (editor
>    philosophy — intermediate typing state shouldn't make semantic functions disappear). Discovery
>    arm for Lsp keeps buffer source overwriting disk stale content; Typecheck under Lsp only checks
>    the target file (other units only for registry to provide signatures); in-file diagnostic order
>    normalization principle extends to LSP (typecheck diagnostic → W code warning → proof error).
>    The old behavior of LSP hard-aborting on unrelated disk file parse errors and the handler
>    silently falling back to single-file path quirk is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` new `shared_registry` field — embedded std modules are
>    sub-compilations, the registry is **input** not product (EMBEDDED_STAGES has no Registry
>    stage), #94's SymbolTable sharing contract goes from "caller remembers to pass the same
>    registry" to being explicitly declared by the program. Error path text unified from
>    `<std.test> (embedded std)` to unit virtual path `<std/test>` (only compiler-internal error
>    surface, no test nailed down). At this point, all four orchestrator entries are migrated into
>    Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): Check form is
>     the faithful bearer of single-file semantics for programs without a project root — warning
>     surface only covers the entry file (neighboring files only collect errors, rather miss than
>     false-positive); relative `use` resolves along importer directory (rustc single-file mod
>     alignment). CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path
>     (pipeline full chain) already runs IR generation, E3019/E1014/E1015 etc. only produced in
>     ir_gen (runner gate empirical). IrGeneration's Check form is pure check, doesn't consume IR;
>     Script/Bin sub-forms expressed as module_key's None/Some (E3023's existing switch,
>     ir_gen.rs:1660). Monomorphization doesn't enter CHECK (only produces resource exhaustion /
>     internal error, zero corpus dependency). **〔2026-10-10 retracted by 3.4.8 empirical
>     evidence〕** — "zero corpus dependency" is actually evidence of corpus lacking pathological
>     recursion fixtures; E3005 is the only compile-time defense for this class of programs. See
>     note #14. Side fix of latent defect: Registry arm's re-harvest of registrations for embedded
>     std units would overwrite the native half-face already merged by with_std() (E1043 false
>     report) — embedded units now skip duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence deleted, changed to
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI use the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, keeps incomplete AST and
>     continues typecheck (editor philosophy extended from 4.2.3 ruling; not marked as stage
>     failure, otherwise topological skip would make typecheck never run); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path from this point on complements
>     proof (E4018), W code warnings (W1001–W1005, test nailed down), and IR-level errors; lexical
>     failure changes to reporting real diagnostic (old synthesized "E0001 lexical error" text
>     disappears with sequence deletion — real diagnostic carries precise span, no information
>     loss).
> 12. `check_module` / `check_module_collect_all` per-line verification (4.2.7, C5 mandatory): the
>     two entries have long converged — mod.rs layer is `check_module_inner(ast, env, collect_all)`
>     single bool, checker layer is `check_module_impl(module, collect_all)`; the only fork is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, pass-3 and drain both modes run
>     together, difference carried by whether `collected_errors` is empty; five collection points in
>     statements.rs (function body / use / for / block / while). Verification confirmed and fixed
>     the duplicate diagnostic defect: the collection point puts the first error in both
>     `collected_errors` and Err return channel simultaneously, pass-3's `add_error` for Err causes
>     same code / same span / same message × 2 (nested can be × 3); plus annotation validation
>     signature formal parameter and overall annotation double visit (E1003/E1103), ownership layer
>     same-point double emission (E2014/E2018) two independent mechanisms. Fix lands at module
>     result boundary: deduplicate by (code, span, message) — diagnostics are positional facts,
>     duplication carries no information; 63 corpus entries remove duplicate copies, run column and
>     exit code zero drift (`borrow_conflict_err`'s "duplicate" is a baseline triple-format artifact
>     missing column number — two E2018 are actually legitimate diagnostics at different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` via `compile_playground` constructs
>     `Program { kind: WasmPlayground }` + Driver — the `Compiler` wrapper layer is no longer
>     involved. Error text byte-for-byte aligned with old `CompileError` Display prefixes (Parse
>     error:/Type error:/Internal error:), playground UI zero visible difference. WasmPlayground and
>     SingleFile share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step onward has
>     a real producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partially retracts note #10's mono exclusion**). Empirical evidence overturns "mono only
>     produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) under
>     multi-file path without mono causes the compiler process to stack overflow (0xc00000fd —
>     interpreter recurses in Rust layer, no graceful runtime error), mono depth gate is the only
>     compile-time defense; standalone check before 4.2.5 went through pipeline full chain and had
>     mono, exclusion ruling caused silent coverage regression ("zero corpus dependency" only proves
>     corpus has no pathological fixtures). "Type erasure fallback" assumption voided by RFC-033
>     reflection ruling — `^^List(Int)` needs the real identity of instantiation, monomorphization
>     is the foundation of reflection, not an optional optimization. Four design points: ①typecheck
>     produces instantiation_request for qualified calls (`lib.f(x)`, FieldAccess form), generic_id
>     uses qualified name, same namespace as merged IR (currently only recognizes bare Var,
>     expressions.rs:2455); ②`Stage::ALL` topology changed to Linking before Monomorphization,
>     MULTI_FILE stage table tail adds Monomorphization — consumes merged_ir, aggregates all unit
>     requests (deferred bucket's containing_fn qualified), mono upgraded from single-module pass to
>     whole-program pass; single-file stage table unchanged (no Linking, subsequence property
>     maintained, IR snapshot zero drift); ③CHECK adds Linking (pure merge — E3020 entry check left
>     to RoleClassification, prevent double reporting) and Monomorphization (pure check doesn't
>     consume, same as IrGeneration's Check form) two stages; ④resource protection covers cross-unit
>     mutual recursion with whole-program BFS. Monomorphizer itself unchanged (consuming ModuleIR +
>     request set contract is exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design, land qualified call instantiation
>     collection: `ExpressionInferrer` injects module qualified key table (`use` three-arm
>     registration — normal / alias / item-by-item alias and only SubModule kind exports),
>     `callee_generic_name` via `SymbolTable::qualify` splices the qualified name from the same
>     source as merged IR; qualified call first-path arity criterion tightened (when parameter table
>     can't be resolved, fall to second path taking actual arguments by signature, avoid
>     cross-module same-name function mismatch false report E3018). During implementation,
>     empirically found and fixed in the same round **three mono-side asymmetry defects**: (1) The
>     deletion key (generic name set) and rewrite key (containing_fn + name + span triple)
>     granularity mismatch — sites that can't resolve arguments degrade to symbol requests entering
>     deferred bucket, when container function is non-generic the bucket never drains, but the
>     original has been deleted by name → dangling call (`list_ops.yx:53` runtime E6006 empirical);
>     fixed as restore gate `restore_generics_with_uncovered_call_sites`, using
>     `build_call_site_map` as the single source of fact to determine coverage, uncovered sites'
>     originals preserved. (2) The restore segment `HashSet` iteration order caused function table
>     order non-deterministic (t2 re-review F1) → changed to `Vec` and sorted by name, product
>     byte-level reproducible. (3) Non-rewrite surface form TailCall / MakeClosure originally looked
>     up the map key (comment claimed conservative) → `call_form_is_rewritten` single-point
>     judgment, non-rewrite form uniformly counted as uncovered. Each of the three has its own nail
>     test (red-state empirical discriminability before fix). Corpus diff baseline regenerated with
>     the same commit is exactly one line (remove_at frame supplement `(int64)`). **Commit strategy
>     note**: when typecheck collection and mono fix are split into two commits, the staged tree
>     containing only the former triggered `stdlib_docs`'s red on HEAD's existing defect (same
>     family E6006 as `list.is_empty([])`) — the two halves are each other's green light premise, so
>     merged into one atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **D2 ruling A** — user module
>     exports carry `generic_fn_type_params_snapshot()` along with `Export.type_params`, same
>     structure as std path (`yx_sources.rs:62/108`): declared name becomes the authoritative source
>     for qualified call monomorphization, signature reverse inference downgrades to fallback
>     (merged into R2 scope). (2) **HKT declaration side alive, call side broken** (inferring type
>     argument E1002 / explicitly giving type argument E1010) registered as WBS 3.5 (3 three-level
>     tasks). (3) **deferred bucket in non-generic container never drains** merged into 3.4.8 ②
>     acceptance sub-item: bucket must drain or loudly error, cannot silently not specialize. (4)
>     **Closure variable by-name call runtime E6006** registered as WBS 3.6; semantics (i) support
>     indirect call / (ii) compile-time reject, **ruled (i)** — support indirect call: `CallDyn`
>     mechanism is in place (`ir_gen.rs:7945` closure expression path), the broken part is `Var`
>     callee being degraded to by-name `Call`, and typecheck originally accepted this program. (5)
>     **Tradeoff criterion solidified** — scheme rejection reasons can only be based on correctness
>     and readability, "large change surface / new mechanism introduced / high cost" doesn't
>     constitute a rejection reason, write into `coding-rules.md` part two and HOWTO self-check
>     table; tradeoffs needing user decision give all four items per `AGENTS.md` "When Requesting
>     User Ruling (Mandatory)" (situation+evidence / per-item pros and cons / clear
>     recommendation+cost / destructiveness calling out).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     Linking arm **before taking each unit's IR** aggregates the whole program's requests and
>     qualifies `containing_fn` (otherwise deferred bucket key and merged IR are not from the same
>     source, bucket never drains). **① Closure (D2 ruling A)**: user module exports carry the
>     declaration-time type parameter snapshot along with `Export::type_params`, same structure as
>     std path (`yx_sources.rs:62/108`). **Existing defects empirically found and fixed during the
>     process**: (a) **F1** — `qualify_names` only rewrites function body, `ir.init` is never
>     visited, multi-file function definitions already qualified but module top-level calls still
>     bare name → runtime E6006 (**non-generic reproduces**, unrelated to generics); fixed as init
>     reuses the same `rename`/`aliases` and the same `rewrite_call_names`. (b) mono side
>     "whole-program unique suffix" heuristic and miss silent `continue` → delete heuristic, miss
>     change to **loud E3018** (ambiguous silent mismatch worse than not parsing). **Regression
>     caught and fixed by independent verification**: t6 found that after named import
>     `use lib.{pick}`, bare-name call was falsely rejected by E3018 (IR instruction name is alias
>     qualified name, request name is bare name, aggregation side only qualified by unit key) →
>     fixed as aggregation side **directly calls ir_gen's** `import_aliases_from` and
>     `resolve_call_target` (extracted as pub single point, one place produces one place consumes),
>     and pinned with matrix tests covering two forms of named imports. **Filing**: init segment
>     `DefId` blank empirically confirmed as "consistency blank" not living defect (codegen name
>     fallback can resolve, four classes of init calls empirically pass), registered as WBS 3.7.4 to
>     close the caliber fork of "function body via def, init relies on name".
> 18. init segment DefId caliber fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure), function
>     body and `ir.init` share the same implementation and the same `resolve` closure; after 208 IR
>     snapshots regenerated, only 309 `(def=_)`→`(def=dN)` changes (other-lines 0; whole-tree init
>     segment 0 remaining `(def=_)`). The two states of codegen's DefId main path and name fallback
>     produce translation results and execution values consistent item by item (four classes of init
>     calls empirically measured), corpus baseline zero drift; probe flipped and strengthened per
>     new caliber (identity + exact index + execution value, fallback init coverage goes red).
>     **Erratum (2026-10-11)**: The above commit numbers were rewritten by `git pull --rebase` — the
>     original b554497b/e60dcc87/b7ac7396 correspond in order to 375789d2/d922a551/38bd4b9e after
>     rebase; the pushed dev history uses the new numbers (old numbers are local temporary numbers,
>     didn't enter the remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④ design basis see #14; ② landing
>     see #17, this item is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate requests + `link_module_irs`), entry check whole block left to
>     `RoleClassification`; `ir_generation_check` generated and kept `us.ir` (in-memory only); mono
>     failure in Check enters collection channel, in MultiFile keeps `fail_with`; `into_outcome`
>     first `take_check_diagnostics` then judges failure (structural failure no longer clears
>     already-collected diagnostics). **Verification**: four rounds of independent verification
>     (t3/t4/t10 + t5 cross-form matrix) and one re-review (t6) all pass; corpus diff 340/340
>     line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps found
>     and fixed in the same round**: F1 (rootless multi-file check's request name and IR definition
>     key not from same source → silently not specialize, see landing note in WBS 3.4.8 row) — fix
>     is aggregation side accurately reverse-lookup by real definition key (non-heuristic), first
>     version of fix is aggregation side accurately reverse-lookup by real definition key
>     (`flat_defs`); **same-day erratum**: that bridge has been removed in the regression fix, root
>     cause now resolved by `module_key`'s dual-purpose decoupling (`script_mode`), three sources
>     unified to `{unit key}.{name}`. The positioning and fix of this regression (CI doc example
>     gate 258 blocks 10 failed → 0 failed) is in the landing note in WBS 3.4.8 row. **Filing
>     (existing, not this round's regression)**: E3005 help text doesn't match resource protection
>     semantics; multi-file E3005's span label misalignment (HEAD binary reproducible); request
>     collection surface's type parameter position dependency (same surface assigned to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard reachable)**: Interpreted
>     calls changed to **explicit stack-driven** —
>     `CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` after pushing frame hand control
>     back to outer `step_one`, frame carries `ReturnTarget{Host|CallerSlot|Discard}`, on return
>     write back to caller slot, multi-call instructions use frame-internal `pending_calls` to
>     continue. Baseline measured each layer of interpreted call consumes native stack **2544 B**,
>     crash threshold at `count_down(394)` (Windows main thread 1 MiB); after fix 20 000 layers of
>     recursion can run on a 512 KiB stack thread (about 0 per layer). Guard (`push_frame`) is
>     therefore reachable at any entry, any `max_stack_depth` value; `ExecutorError::StackOverflow`
>     changed to carry the real limit (original `util/diagnostic/mod.rs` always passes 0).
>     **Original 3.11.2 (deep stack back-trace truncation) merged into this task by user ruling**
>     (same file, can't be split by file): back-trace changed to first 16 frames + count ellipsis +
>     last 8 frames, shallow stack frame-by-frame unchanged. **3.10.1 (compile-time unguarded native
>     recursion, #447)**: First locate then change — crash point is `MonoType::type_name` recursion
>     (**328 B/layer**, SP probe + per-operation threshold determination), same path also has
>     derived `Clone` (~400 B/layer), `substitute` internal deep copy, `type_name_hash` (164
>     B/layer), `collect_generic_type_refs`; after changing the work list at four places and
>     removing one whole-request deep copy per layer, pathological generic recursion with
>     `mono.max_depth=5000` produces **E3005 (with real limit)**, no longer stack overflow. **3.11.1
>     (task boundary error code fidelity, #448)**: Task result error side changed from value to
>     **structured** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), cross-boundary restore `ExecutorError` by value;
>     task-internal E6001/E6003/E6004/E6008 in Standard mode and Embedded messages are
>     character-by-character identical. Two **pub API payload form changes** are written with
>     `BREAKING CHANGE` in the commit. **Commit strategy note**: 3.9.1 and 3.11.1 **cannot be split
>     at file granularity** — `interpreter/executor/executor.rs` simultaneously carries the
>     trampoline and the error channel's production/consumption points, `TaskFailure` defined in
>     `runtime/engine.rs`, staging only one side won't compile (pre-commit compiles with **staged
>     subset**, measured twice rejected), so per #15's same caliber merged into **one atomic
>     commit**. **Known boundaries and follow-up**: Drop derive (~75 B/layer) ⇒
>     `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate load borne on
>     native recursion (~3 696 B per layer, default 256 layers ≈ 924 KiB), currently zero call
>     sites, production unreachable, future wiring must close the same family; Standard mode task
>     interpreter constructed with default `ExecutorConfig` ⇒ its E6004 limit is always 1024
>     (configuration propagation registered as WBS 3.12, user ruling B: separate team); test runtime
>     budget separately in issue #449 (pathological fixture 76s, clippy dual target, commit runs
>     full), not in this RFC construction surface. **Corpus directory layering (same round,
>     8605a081b)**: 212 fixtures in `02-type-system` (78) and `06-compile-errors` (134) split into
>     23 sub-directories by category, 88 IR snapshots migrated by mirror path; corpus baseline
>     regenerated with the same commit (342 lines, independent recount line-by-line comparison: only
>     `file` field and 2 new changes), 41 snapshots regenerated due to file header path getting
>     longer, line-by-line check **only span offset changes** (zero difference after de-offset
>     normalization).
> 21. LSP semantic data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data
>     produced by the **same compilation**, in-project cross-file references carry `resolves_to`
>     pointing to the real definition — old form was didOpen/didChange triggering **two**
>     compilations: diagnostics went through the unified Driver pipeline, semantic data went through
>     server.rs's manual lex→parse→single-file typecheck (no module registry, in-project `use` parse
>     failure, reference events not produced, cross-file jump / rename / reference lookup silently
>     landed empty — D55 registered silent failure closed in this task). **Implementation**: (1)
>     `Export` carries `definition_span` (extract_module_info harvests from AST identifier node,
>     same source and value as typecheck definition event's `def_span`; native std always None),
>     `ModuleInfo` carries `definition_file` (Registry arm fills by DiscoveredFile); (2) when
>     checker drains reference events via `import_def_locations`, it redirects cross-file import
>     items' `resolves_to` from use line to the real location of the definition file (std import has
>     no registration, keeps falling to use line); (3) checker module identity changed from unified
>     `"<module>"` to unit file path (this value is only consumed on the semantic data surface, zero
>     diagnostic consumption — empirically `env.module_name` all 4 consumption points); (4)
>     `DriverOutcome` adds `semantic_db` channel — CollectAll form (Lsp target file regardless of
>     diagnostic success + SingleFile fallback) merged in, Lsp arm tail from registry injects
>     non-target-file top-level export definition surface (`external_definitions`, unopened files
>     can also jump; types / submodule exports not injected — typecheck side type registration
>     doesn't produce definition events, both sides maintain consistent gap); (5) orchestrator adds
>     `analyze_source_in_project` (`check_source_in_project` redirected and only takes diagnostics
>     half); (6) LSP side `analysis.rs` single compilation entry replaces manual pipeline
>     (server.rs's `update_semantic_db` deleted), World key unified to normalized file URI
>     (`uri.rs`: `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX
>     encoding difference normalized to same key). **Side fixes (D59 split class B — old behavior
>     silently no-op, doesn't change any originally successful program's external behavior)**:
>     in-project files' cross-file jump / rename / reference lookup from silently empty to hitting
>     real definitions; on lexical error, old manual pipeline deletes semantic info, changed to
>     incomplete AST staining preserved (4.2.6 editor philosophy extension); on target file with
>     type error, semantic data no longer discarded along with `type_result` (staining / jump not
>     disappear in intermediate typing state). **Nailed down**: driver-level 6 (target file path key
>     / cross-file resolves_to+external definition surface round trip / std import fallback to use
>     line / error file preserved / single-file fallback path key+error preserved), LSP e2e 1
>     (red-then-green empirical — before fix jump returned None), URI normalization 4.

**Transformation approach for the ten entries (function by function)**:

| Entry                                                      | After Transformation                                                                                                                                                                                                  |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                        |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                               |
| `compile_project` (`orchestrator.rs:99-237`)               | Slimmed to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                           |
| `check_project` (`orchestrator.rs:273-399`)                | Slimmed to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                             |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filtered to target file                                                                                                                                                            |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                             |
| `lib::run_file`                                            | Unchanged (forwards to `run_project` or `SingleFile`)                                                                                                                                                                 |
| `lib::run_project`                                         | Unchanged (forwards to Driver)                                                                                                                                                                                        |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changes to `Program { kind: Lsp, aggregation: CollectAll }`; **the manual lex→parse→… sequence in single-file branch is deleted**, changes to call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                         |

**Duplication degree between `check_project` and `compile_project` (about 40% shared / 60% fork)**.
What's shared: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parse loop, per-file checker assembly, result handling. What's forked: items
2/3/4/5/7/8/9/10/11 in the table of 11 above. **This 40% shared skeleton is exactly the value zone
of `Driver`** — the forked 60% is all "different stage set" or "different aggregation mode", and
those are exactly the two fields of `Program` that can completely express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "skip due to upstream failure" is not a stage return value.**

Reason: skip is **topology**-decided, not stage-self-decided. If each stage returns `Skipped`
itself, the information "why didn't I run" is scattered across 12 stages and cannot be centrally
audited. Changed to have `dispatch` decide at the loop start by dependency:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
is fixing — the `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip" that produces
no diagnostic, just because the obligation happens to be empty. After the rule is established, any
"didn't run Y because X wasn't done" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                    |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                               | "Stage Y did not execute because upstream stage X failed"                                                                         |
| Caused by empty pre-obligation                           | "Stage Y did not execute because no pending obligations (normal)" — **this item's severity = Info, not counted in warning count** |
| Conditions not met (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because configuration not enabled"                                                                       |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic flow, and doesn't pollute `yaoxiang check`'s `warning_count` (`diagnostic/mod.rs:637-641`
depends on this count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error means abort (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)              | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a `Program` field, not a Driver global setting** — LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "early exit or not" —
they are two different checker entries. After unification, it should be `Aggregation`
parameter-driven same implementation, not retaining two functions (**this is an additional item this
document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| New `driver/` module                        | **None**. No new type representations introduced, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                             |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only attribution changes. `release_plan` still `ownership::ReleasePlan`, `proof_calls` still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                       |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type attribution and layering**: what migrates into the ledger is **settlement
responsibility**, not type attribution module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay where they
are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g. `ir_gen` reads `release_plan`) continue to receive specific field types via parameters, **must
not `use crate::driver`** — this is consistent with the driver's non-reverse-dependency red line
below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**.
`Driver` appearing in `TypeChecker`'s import is treated as a violation, intercepted by
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` for standalone no longer goes through this pipeline — uniformly changed to `compile_project` (MULTI_FILE stage table), Script semantics driven by rootless; this line now only describes `lib::run_file` / `Compiler::compile` library path) |
| `yaoxiang run` (multi file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already connected in P3 stop-gap; **warning surface doesn't add new** (decision B1, 2026-10-09: dead code family warnings don't enter compile path — pool semantics being corrected by 4.9 defect judgment, unified warning surface left for after 4.9; this line's draft "new warning output" voided)                 |
| `yaoxiang check` (multi file)       | No proof_execution                       | Already connected in P3 stop-gap                                                                                                                                                                                                                                                                                                       |
| LSP (in-project)                    | No proof_execution                       | Already connected in P3 stop-gap                                                                                                                                                                                                                                                                                                       |
| Z3 not installed + single file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                             |
| Z3 not installed + multi file       | Silently skipped                         | Same as above, unified                                                                                                                                                                                                                                                                                                                 |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This conforms to RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract `backend.rs:60` has declared ("caller should conservatively degrade") — the
current implementation is contrary to its own contract document.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (single file 1 / multi file N)                                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                                      | Change                                                                                                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                       | `140-147`                                       | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `154-167`                                       | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `141-227`                                       | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `275-278`                                       | Dead code analysis removed, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `306-311`                                       | `run_proof_execution` **kept**, moved into `driver` as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `381-389`                                       | Monomorphization removed, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                        | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                       | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `450`                                           | `check_source_in_project` goes through Driver                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `1374`                                          | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                       | The two-way choice between `check_module` / `check_module_collect_all` in `typecheck_with_registry_in` changed to be driven by `Aggregation` parameter                                                                                                                                                                                                              |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                       | `run_diagnostics`'s manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                                           |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                       | `check_files_with_diagnostics` uniformly goes to `Program { kind: Check }`                                                                                                                                                                                                                                                                                          |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                       | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                     |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                                | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently via `Compiler::compile_with_source` implicitly falls to path 1)                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                         | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                     | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent discard point**)                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                          | Correct comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/layers/README.md`     | Full text                                       | Layer order table changed to **actual** execution order; delete "lower layer fails, upper layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                         | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumer points `predicate.rs`/`checker.rs`/`ownership.rs` changed to go through the shared port. `&'static` raw reference form is infeasible (`dyn Solver` is not `Sync`), the original text is governed by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logic itself is
correct — **the problem is at the consumer end not the producer end**, changing the producer end
would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch
(reachability judgment of wasm branches assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                          | Disposition                                                                                                                                       |
| --------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| New proof_execution on multi-file path                    | **Breaking**: previously silently-passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; batched release, expected diagnostic set changes go through CHANGELOG |
| New W1001/W1002/W1003 on multi-file path                  | **Breaking**: programs that compiled clean now produce warnings        | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                             |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: new diagnostic set                                       | W first then E, S2/S4 two steps                                                                                                                   |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                     | No break                                                                                                                                          |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                          | These two paths don't go through proof_execution                                                                                                  |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactor**                                                  | `pub` API surface needs sync if there are external dependencies; repo-internal consumers are fully listed in the change list                      |

---

## Implementation Points

This document corresponds to **P3 (fix correctness vulnerabilities)** and **P4 (stage contract and
unified Driver)** of RFC-039's global stage sequence. The following S1-S5 are the implementation
order within this document, **each step's acceptance criterion cites the C2 category of the
[Equivalence Criteria Document](07-equivalence-oracle.md)** (orchestration change: same **diagnostic
set** per entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must first be written as failing.

| Deliverable                                                                                                                                              | Acceptance                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                            | **Must be red**. If accidentally green, it means this document's vulnerability analysis needs re-verification              |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48 — not mixed into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Single-file and multi-file versions of corpus diagnostic sets can be compared                                              |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                           | Mark `#[ignore]`, S4 turns green (red state left on file: after removing the single settlement point `pairs: 0`, 6 failed) |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-for-byte identical**                    |
| Whole corpus (293 `.yx` files) diff                      | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately doesn't fix any bug** — it just moves existing behavior into Driver. The
acceptance criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restores.

### S3: Merge Four Orchestrator Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four entries' diagnostic sets normalized by `Aggregation` are identical           |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in-project and out-of-project               |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give same diagnostic set for the same file                            |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics when Z3 is not available                                         |

**Rollback point**: vulnerability fix and structure merge are **two commits**. If structure merge
has problems, only structure commit can be rolled back, vulnerability fix commit preserved — at this
time `test_multifile_proof_obligation_not_dropped` stays green; reverse (keep structure, roll back
fix) goes red, is unacceptable intermediate state, forbidden to merge.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                                   |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 use cases turn green (implementation form see §2 revision note; use case count is subject to that file) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                                     |
| Obligation diagnostic W → E escalation       | Manually review every newly added E after whole corpus diff                                                                                  |

**Rollback point**: `assert_drained()`'s severity can be controlled by a config item, W/E switch
doesn't need to change code structure.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                 | Acceptance                                                                                                                                                             |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                      | Documentation corresponds item by item to `checker.rs` call points                                                                                                     |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown doesn't enter cache** — otherwise one timeout solidifies across compilations via process-level cache, and pollutes cargo test thread-shared) | Production path's `default_solver()` call sites go to zero (only tests retain); cache hit rate observable across three consumers (counter already landed in 89576fafd) |
| `checker.rs`'s `TypeChecker::ownership_inputs` criterion points (ownership block `:1296-1340`, emission point `:1531`) degradation adds warning                                                                             | Has diagnostic when no `body_checker`                                                                                                                                  |
| Reachability judgment of 10 wasm attributes (4 positive + 6 negative) in `orchestrator.rs`                                                                                                                                  | Conclusion given to "F2 Judgment" subsection of `06-cleanup-inventory.md`, this document only registers the judgment requirement                                       |

**Note**: correcting layer order will change the diagnostic set, may expose large amounts of
previously-silent `Unproven`. **This item should go independently of S1-S4**, don't mix with entry
merge in the same PR.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                      | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustible enum, **runtime registration refused**                                                                                                                                                                                                                                                                                                                                                       | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant, `dispatch` doesn't compile. This is isomorphic to the mechanism already validated by the opcode table in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating the need for human-reasoned call count issues                                                                                   |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used in parallel with **static** gate of `scripts/ci/check-obligations.py`; **final state severity E level** (2026-10-13 user ruling A: same code escalated to E, code number unchanged; **implementation form see §2 revision note**: ledger doesn't copy field structure; red list — after clearing 0 entries — mechanism kept, doesn't enter user-visible diagnostic stream) | What's being fixed is a whole class of bugs, not one: at least 2 same-class hidden dangers already verified ( `proof_calls`, `checker.rs:1313`), all 13 obligation fields are in range. W→E two-step is to make every step's C2 criterion usable — now the obligation list has been cleared to 13 entries, red list is zero, healthy state zero emission, so final state takes E directly (see §2 revision note paragraphs seven, eight) |
| **Fix scope**            | Fix the **consumer end** (orchestration layer), **don't touch** the producer-end logic of `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branches                                                                                                                                                                                                                                                                                  | The problem is at the consumer end not the producer end. Changing the producer end would mask the architectural defect as "logic fixed", but the logic of these three branches is itself correct                                                                                                                                                                                                                                         |

This design additionally eliminates two types of cracks:

- **The crack between "comment commitment" and "code behavior"**. The "do not let silent pass be
  resurrected" at `checker.rs:5165` is a **comment-level contract**, fulfilled by
  `assert_drained()`.
- **"Forgot to append in an array"**. Currently five functions each hand-write the call sequence;
  after unification, there's a single stage table, and missing to wire a new stage will cause
  compile-time error.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of the 11 inconsistencies
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, are "should we run some analysis"
  configuration issues; it also can't express `Aggregation`, can only move the two-way choice of
  `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, equivalent to
  trading "five functions each hand-write calls" for "forgot to append in an array", which is
  exactly the cause of the current 11 inconsistencies. The repo already has same-form
  counterexamples: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implementation.
- **Only patch bugs without touching structure** — can fix the known 2 bugs, but only covers 1 of
  the obligation fields, `Stage` is still scattered across 5 functions, the next fork point will
  keep growing from here. **It must be done first** (is part of S1/S3), because structural
  transformation needs a known red test to prove the criterion is valid.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` can't hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is looking for a bug at the wrong layer:
  the bug is in the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After 139 lines of
  `compile_project` are split into "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent problems"**. After fix, a batch of "new errors" user reports may appear — they
  are real bugs, just never reported. Must be clearly stated in CHANGELOG.
- **`assert_drained()`'s severity switch needs per-field manual judgment**. Among obligation fields,
  some (e.g. `module_namespaces`) having "no consumer" may be by design not needing consumption,
  shouldn't alarm. S4 needs to go through field by field, can't take a one-size-fits-all approach.
- **`Program` abstraction may be premature**. If some entries' stage sets are long-term unstable,
  `stages()` will degenerate into "pass different array every call" free parameter, contract
  constraints fall through. **Mitigation is `test_program_stage_coverage` asserting `stages()` can
  only come from 6 predefined combinations.**
- **Criterion dependencies**: S1/S3's vulnerability fix criteria depend on
  [Equivalence Criteria Document](07-equivalence-oracle.md)'s vulnerability test being red first.
  **If that document doesn't establish the multi-file corpus layer first, S1 cannot be accepted** —
  because the existing 293 corpora all go through the single-file path, zero coverage for this class
  of defect. This document doesn't involve the IR verifier (`verify_loose`), that part of
  prerequisite work is not in this document's scope.
- **Layer order correction will expose previously-silent `Unproven`** (`equivalence` isn't even in
  the pipeline, `termination` and `ownership` order is opposite to declared). This is a diagnostic
  set change risk, should go independently.
- **Thread safety after SMT backend changed to singleton is undetermined**. `backend.rs:13-19`
  already states `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after
  changing to cross-compilation-unit shared singleton, need to confirm access paths all go through
  `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (e.g. termination and ownership's actual
  order dependency). Should be opened after equivalence criteria stabilize.

> **All open questions originally listed in this section have been ruled on.** Per-item decisions
> see [RFC-039 Ruling Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document has no remaining undecided items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing table A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactor (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "do not let silent
  pass be resurrected"
- `src/frontend/core/typecheck/layers/README.md` — Layer order call table and `equivalence`'s real
  identity (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — "Caller should conservatively degrade"
  contract declaration (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 clearing)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test self-admits
  "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus self-admits "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs`'s `compile_playground` (`run_code` compiles through it) — playground's fourth
  compilation caller (single-file path, so executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate not main crate
- `build.rs:19-55` — Error code build-time gate, the example of this project's enforcement mechanism
