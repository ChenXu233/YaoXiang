# Compilation Stage Contract and Obligation Ledger

> **Subsidiary design document.** This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage order are in the RFC-039 main
> text; the positioning of each subsidiary document is in [this directory's index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
problem: **"which stages ran in this compilation, which didn't, and why"** is currently scattered
across six paths and ten entry functions, each manually wired, with no single place that can answer
this question.

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are violated by the architecture, not by the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` originally states "Unproven →
compilation error, no degradation, no silent pass… no resurrection of silent pass". But this
invariant only holds on the **single-file path**: proof function constraints like `x: Sorted(3)`
silently pass on the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP—the proof
function is never executed, and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, non-runtime registered) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-function refactoring approach for the ten
  entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-regression gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 gives the **why** for the refactoring and the **order** in which to do it; this document
gives the **concrete form** of L1, the **per-file change list**, and the **internal implementation
stages** (corresponding to P3 "fix correctness bugs" and P4 "stage contract and unified Driver" in
RFC-039's global sequence). Where this document conflicts with RFC-039, RFC-039 takes precedence.

The equivalence criteria are executed under the **C2 (orchestration changes)** category in the
[equivalence criteria document](07-equivalence-oracle.md): same diagnostic set per entry + same
corpus behavior.

## Current State

> This section is entirely **verified facts**, each with file path + line number. Line numbers are
> based on `9e02e4db`.

### Stage Boundaries Are the Only "One Mistake, Whole Repo Silent" Structural Defect

It differs in nature from the other two defect types (module boundaries, test wiring):

| Defect                  | Typical Manifestation               | Is There a Signal?       |
| ----------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntactic error | Source code written wrong           | Has diagnostic           |
| Type mismatch           | Type written wrong                  | Has diagnostic           |
| Stage missed wiring     | A stage is simply not called        | **No signal whatsoever** |
| Obligation not consumed | Field filled in but no one reads it | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. Therefore, they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
is written, not **what is not written**. This is precisely the root cause diagnosed by RFC-039:
"This project treats 'design' as a documentation convention rather than an executable constraint".

### Existing Enforcement Mechanisms in This Repository

The same repository already has mature enforcement mechanisms; they just haven't been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** by `build.rs:19-55` through `tools/code-tables`, which compares each one
  against the RFC-013 code table line by line, and any inconsistency causes immediate `panic!`
  refusing to compile.
- `src/package/` (**76 files / 13,012 lines**, of which the `tests/` directory has 6,227 lines,
  47.9%) has high test density, with each module's test subtree being declared-wired—this is the
  most completely test-wired block in the entire repo, and can serve as the form reference for the
  stage layer gate.

**Design capability is sufficient. What's missing is "putting a same-level gate at the orchestration
layer too".**

(Line count convention: `(Get-Content).Count`, see the "Line count convention" section in
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                 |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                              |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                  |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking            |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics       |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR          |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1                 |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                     |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes to 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                     |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1          |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)—RFC-027 Phase 2.5                   |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Places of Stage Coverage Inconsistency

The table below provides evidence cell by cell. **A blank does not mean "this entry doesn't do this
thing", but "there is no code in this entry that does this thing"**—this is the very essence of the
defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                       | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **No**                                                  | **No**                                                      | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **No**                                                  | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                           | **No**                                                  | Yes `336-348`                                               | **No**                             | **No**                                |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                          | Yes `350`                                                   | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **No**                                                  | Yes (same as 2)                                             | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **No**                                                  | N/A                                                         | N/A                                | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                              | `discover` (`101`, discards `used_by`/`shadow_events`)  | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                           | **No**                                                  | Yes (`321-328`)                                             | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | N/A                                              | Yes `allocate_global_slots` (`147`)                     | **No**                                                      | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`205`)                                      | Yes (`152-236`)                                         | **No**                                                      | **No**                             | Yes (independent ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, kept without changes. The current state differences after P3
> is implemented:
>
> - **Row 1 fixed**: proof_execution is shared by all five entries using the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the sole-consumer defect
>   no longer exists.
> - **Rows 2/4/5 remain unchanged** (multi-file `run` still doesn't output W1003 or run dead code
>   family) → attributed to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file only) → attributed to WBS 3.4.8 (prerequisite 4.1.3,
>   potential risk unproven defect).
> - **Row 7** check_module / check_module_collect_all dual entry → attributed to 4.2.7 (Aggregation
>   parameter driven).
> - The "#434 ruling" referenced on the code side was previously unregistered in docs—now registered
>   as RFC-039 **D57**.
> - **Embedded std entering asymmetric coverage (new fact outside the table, registered
>   2026-10-09)**: The single-file path injects std.list unconditionally via `merge_embedded_std_ir`
>   (required for for-loop desugaring, #117 hard switch); multi-file `discover` previously only
>   recognized explicit `use`—in-project for loops compile cleanly but cause runtime E6006
>   (empirically verified by probe). Fixed and crossed off WBS 4.10.2; the same family "multi-file
>   missing one step from single-file" parse hard-abort quirk logged as WBS 4.10.1—**also fixed**
>   (implemented right after 4.2.2; Check path downgraded to per-file collection; this audit method
>   looks at "stage coverage/field consumption" dimension, this is a compilation unit membership
>   difference, an escape dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   The Registry arm uses `extract_module_info` to repeatedly harvest registration for embedded std
>   units, which overrides the native half registered by `with_std()` under "native + yx surface
>   merge" (`result.is_err` and the like are lost → std.test falsely reports E1043). This is the
>   third escape dimension of this audit: "the merge semantics of two registration sources for the
>   same module key"—the single-file path's registration table is formed only once through
>   `with_std()`, while the multi-file/Check Registry arm inserts per unit to achieve coverage.

**Two places require precise expression, otherwise mistakes will be made:**

- **The precise conclusion of rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  at `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function has **no read site of `result.warnings` anywhere**; the
  only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion is also from different sources**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  checks "whether there's a manifest"). The two functions give different answers to "which files
  must define `main`".

### Correctness Bug: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation is produced at exactly one place.**
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

The semantics: "the predicate argument of this refinement constraint is a compile-time literal; the
proof function must actually be executed to determine it".

**Step 2 — The consumption site is inside the checker, but it only emits without consuming.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `Therefore this branch becomes a real defense line rather than forward-looking placeholder: no resurrection of silent pass.`

That is: the author knew "not producing diagnostics" was a defect, classified it as a **known,
nowhere-to-go intermediate state**, and assumed by design that "someone will definitely read
`proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects; `checker.rs:1439` writes it into `TypeCheckResult` with
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is **only read at `src/frontend/pipeline.rs:187`** in production code (passed as
argument at `189`):

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
`verdict.rs:61` is the same-named field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries bypass that layer.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), `compile_embedded_module` (`1374`) **all do not call
`pipeline.rs`**, directly calling `TypeChecker::check_module` (`124` / `493` / `1386`). Therefore
they cannot even reach the only read site at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations also go through the
dropping path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible
for compiling embedded std. This means **the proof obligations of the embedded std modules
themselves are also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP paths **compiles, runs, and
produces no diagnostic**. The proof function was never called.

**Step 8 (supplementary verification) — There is a second silent drop point within the layer.**
`checker.rs:1303-1314` handles ownership layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by the empty match arm, with no diagnostic and
no accounting.** This directly contradicts the promise of "no resurrection of silent pass" at the
`5164` branch, and is **independent of the orchestrator issue**—even if the entry layer is
completely fixed, this one will still be silent. (Processing timing: should be handled in the same
batch as the obligation ledger, because it is the same class of problem as the obligation ledger.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits for the keywords `Sorted`
/ `proof` / `refin`.** The multi-file path has zero coverage of proof obligations.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping just before `pipeline.rs`**.

The test file's own documentation comment already states this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation test cases at the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation is filled in", and the
bug is "the consumer doesn't read it".** A test that only tests the producer side and not the
consumer side is naturally immune to this class of defect.

### A Stronger Discovery: The Equivalence Criteria's Main Corpus is the Single-File Path

The [equivalence criteria document](07-equivalence-oracle.md) uses the end-to-end differential of
293 `.yx` corpus files in `tests/yaoxiang/` as the **third-layer criterion** and the primary
acceptance method for C2 stages. But in practice:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (full directory glob returns
  zero hits).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for each corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  and all don't cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is
empirical proof of this point. The file is annotated with `// expect: compile-error E4018` (line
15), and its header comment on line 7 says "Status: ❌ should be rejected at compile time"—**it can
pass precisely because it goes through the only path that actually executes the proof function**.

**Conclusion: The third-layer criterion needs to add a multi-file corpus layer; otherwise it cannot
serve as the acceptance tool for this document.** See implementation point S1.

### Obligation Fields: 13 "Produce Equals Contract" Fields, Zero Mechanism Guarantee

Of `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, the documentation comments for
**13 fields** explicitly state "produced by stage X → consumed by stage Y", meaning they are
essentially **cross-stage obligations**:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Row-fetching convention**: Definition line = the **declaration line** (`pub` / `pub(super)`
> line) of the field in `types.rs`, not the first line of the doc comment—because the comment block
> for the same field may span multiple lines, and the declaration line is the only anchorable
> position. The **structure range** mentioned in the sentence above this table
> (`src/frontend/core/typecheck/types.rs:16-76`) floats with the structure's line number, and is
> mechanically reconciled by gate **F5** (red on error); the definition line column is reconciled by
> **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | | `release_plan` | `38` |
> ownership → IR generation (**`PlanId`-keyed**: read at `ir_gen.rs:360`
> (`type_result.release_plan.clone()`), used at `:3103` (`ReleasePlan::consume`); settlement
> completion marker `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId`-keyed**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement completion marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Reclassification record (3.4.9, 2026-10-13 user ruling A)**: The original 14th row
`interface_impl_registry` (`types.rs:49`) has been **reclassified as in-tree fact**—it is consumed
by typecheck in-phase as designed, with zero cross-stage reads; it is not an obligation:

- **At phase consumption sites (live)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), seven constraint-solution queries in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), `checker.rs:214-217`
  read-only accessor, `checker.rs:974` injects into body_checker.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side only the ledger's own registration and documentation (removed in
  this round).
- **Disposition**: Remove the **declaration** "mirrored for middle/LSP consumption" from the doc
  comment, retain the field and its in-phase consumption; this field is registered in
  `NON_OBLIGATION_FIELDS` of `src/driver/obligations.rs` (with reason). If LSP really needs this
  table in the future, register under 4.6.x; **do not create a consumer in this round**.

**All these fields are passed across layers keyed by `Span` or name table.** The equivalence
criteria document has pointed out that span-keyed contract mismatches "silently fail with no error"
(`ReleasePlan` is typical: any inconsistency in span calculation on either side → `Drop`
instructions silently disappear).

Why does `release_plan` survive while `proof_calls` is dead? The difference is **structural, not
accidental**:

- `release_plan`'s consumer `ir_gen.rs:360` (`release_plan: type_result.release_plan.clone()`) is
  inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in the downstream module, and the downstream module is
  on the necessary path of all entries.
- `proof_calls`'s consumer is at the **top level of `pipeline.rs`**, belonging to **another entry
  implementation**. The four orchestrator entries don't go through `pipeline.rs` at all.

**Verified fact**: Currently the entire repository has **no mechanism** that guarantees these 13
fields are consumed. The only "protection" is that `ReleasePlan` happened to piggyback on IR
generation.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Fields in `TypeCheckResult` that are not cross-stage
obligations are registered one by one (field name + reason), **complementary** to the "§Obligation
Fields" table above:

| Field                     | Why It's Not a Cross-Stage Obligation (Key Points)                                                                     |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit identity metadata (0 hits in `src/middle`)                                                                        |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produce equals contract")                                             |
| `bindings`                | Has cross-stage reads (ir_gen.rs:735/:5034), but doc comment doesn't declare a cross-stage contract ⇒ not enrolled     |
| `local_var_types`         | Used for diagnostic rendering (ir_gen.rs:852/:4543/:4794), same as `bindings`—no declared contract                     |
| `semantic_db`             | Consumer is in LSP (src/lsp/server.rs:493), not a compilation stage obligation                                         |
| `trait_table`             | Cross-stage reads at ir_gen.rs:4336/:6867, doc comment doesn't declare a contract                                      |
| `warnings`                | Warning diagnostic channel (same kind as `diagnostics`)                                                                |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption sites see the reclassification record in "§Obligation Fields") |

**Why not "wire up the consumer side"**: Creating a downstream for these fields would only introduce
false obligations and disguise "by design not needing cross-stage consumption" as a contract defect;
the correct closure mechanism is **reverse containment**—`TypeCheckResult` field set = obligation
list ∪ non-obligation registration (hard-enforced by `scripts/ci/check-obligations.py` criterion 1:
add field and forget to register, delete field and forget to sync, non-obligation registration
without reason, two tables overlap—all turn red immediately).

### Four Additional Contract Defects in the Proof Layer

All the following come from the proof layer analysis, independent of the entry branching problem,
but all belong to the class of "stage contract not being enforced".

**(a) The layer order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependency    |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "execute in layer order, lower layer failure prevents upper layer from running".
The actual call sites in `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** from the declaration; the
declared Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all**
(it is only used by `inference/assignment.rs:17` as the `is_subtype` utility function, unrelated to
`ProofResult`). At the same time, **there is no short-circuit**—`checker.rs:1271-1276` `add_error`s
termination's errors one by one and continues down; the README's promised "lower layer failure
prevents upper layer from running" doesn't hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-fail panic has been eliminated by P3's 3.3.1 (SOLVER slot
> `Option`-ified, missing falls back to `SMTResult::Unknown` conservative degradation); "silently
> skip (don't inject)" was supplemented with W1081 signal by 3.4.2. The unification of the three
> forms (singleton-ization) is executed per **RFC-039 D58**, with the actual fix location at
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure access
> point), and the `default_solver() → &'static` expression in the 02 change list is subject to D58
> (the bare `&'static` reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                                | Acquisition Strategy                                     | When Solver is Unavailable                                |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Constructed-time injection `with_solver_owned`           | `None` → **silently skip** (don't inject)                 |
| `ownership.rs:627` (back-edge cut determination)        | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively don't cut |

The comment at `predicate.rs:31-32` explicitly chooses hard-fail: "Initialization failure maintains
**hard-fail**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". While `backend.rs:60`'s contract documentation says "`None`: backend
unavailable……**caller should conservatively degrade**". **Two philosophies coexist in the same
codebase, and the hard-fail one panics instead of returning an error.**

**(c) Each back-edge determination creates a new Z3 context, the cache is effectively useless.**
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

**This is a factory function, not a singleton**—each call does `Z3Backend::new()`, i.e., creates a
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this calling pattern**.

(Comparison: `predicate.rs:34-36`'s `LazyLock` is indeed a singleton. Same backend, two lifetime
strategies.)

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
table** and runs as usual—ownership analysis degrades to "nothing conflicts", **without any
warning**. (Suggested handling: should be recorded as a warning-level diagnostic, or at least left
as a trace in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type check and before constraint
solving". The actual `self.env.solver().solve()` is at `checker.rs:1320`—**termination is at `1256`,
ownership is at `1296` after that, i.e., the "before" in the comment is actually "after two
layers"**.

### wasm Current State: Carried by shim crate, the 27-file branch is alive

The wasm target **has been built and is built in CI**, with `cdylib` not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                               |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                               |
| shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                      |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                  |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                         |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from fixed URL, warning if missing)                                                                                                                                                                            |
| **27 files** contain `wasm32` literal             | of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in docs                                                                                                                                      |
| `orchestrator.rs` 20 places                       | 12 attributes + 8 comments                                                                                                                                                                                                                                                       |
| `lib.rs` 11 places                                | all attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm
target—`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole (these
three all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly related to this document**: The playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path that consumes `proof_calls`           | Whether the proof function is executed |
| ------------------------------------------ | -------------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                                |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                                |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                                |
| Multi-file `run` (`compile_project`)       | **No**                                 |
| `check` (`check_project`)                  | **No**                                 |
| LSP in-project (`check_source_in_project`) | **No**                                 |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in playground
scenarios—goes to `06-cleanup-inventory.md`. The open question in RFC-039 about "build wasm target,
or delete these branches" **has expired**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Non-Runtime Registered)

**Core constraint: `Stage` is a compile-time exhaustive enum, runtime registration is prohibited.**
The reason is that Rust's exhaustive `match` can force at compile time that "new stages must be
handled by the orchestration layer"—this is exactly the mechanism proven effective by the opcode
table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check       Project
    Discovery,          // File discovery                 Project
    Parsing,            // Lexical + syntactic            PerModule
    Registry,           // Module registry construction   Project
    RoleClassification, // Role classification (Script/Bin/…)  Project
    Typecheck,          // Type check (with embedded proof layer)  PerModule
    DeadCodeAnalysis,   // Dead code family analysis      Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation         Project
    IrGeneration,       // AST → ModuleIR                 PerModule
    Monomorphization,   // Monomorphization               Project
    Linking,            // Cross-module linking / IR merge  Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array and the exhaustive match
    /// in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once at
project level) make "which stages must run per module and which must run once at project level" a
type-level fact. Project-scoped stages are structurally guaranteed to run only once in
`dispatch`—this eliminates the current need to reason about issues like "whether
`allocate_global_slots` should be called per file" in `orchestrator.rs`.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on the order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the initial draft,
> corrected by implementation evidence—
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the initial draft order contradicts
>    data flow.
> 2. The Check form's stage table is normalized to dead code **before** proof according to
>    `Stage::ALL` topological order: the current `check_project` state is proof before dead code
>    execution, with no data dependency between them and the same diagnostic set (C2 set semantics),
>    only intra-file diagnostic order is normalized. The single-file path (dead code embedded in
>    typecheck, before proof) already conforms to ALL order, byte-by-byte acceptance unaffected.
> 3. `Parsing` moved **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    initial draft order would force the Registry arm to "hide parse", making the stage table lie
>    about data flow; and multi-file parse is reduced from 2 to 1 time as a result. The current
>    state honestly registered as a corollary: the multi-file path's parse error was previously a
>    hard abort (`?` propagation in `build_registry_from`, the tension with CollectAll semantics
>    registered as WBS 4.10.1)—**4.10.1 has been fixed** (2026-10-09 user ruling, rust-style
>    collection semantics + plan B): Check path parse failure downgrades to per-file diagnostic
>    collection, sick files exit the compilation unit (don't enter registry, importers report
>    E5001); MultiFile path retains hard abort (correct semantics under FailFast where bad files
>    can't produce IR, nailed long-term).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data dependency
>    edge 14→16: `RoleClassification` consumes Discovery's used_by edge set, `DeadCodeAnalysis`
>    consumes Discovery's W1006 shadowing event—the initial draft missed these (these products were
>    produced by `discover_with_used` in the orchestrator era, wrapped and discarded by `discover`,
>    the data flow was lost from the table); b. Intra-file diagnostic order normalization extended
>    to the full form of note #2: E3020 entry validation moved into the RoleClassification arm
>    (Check has no Linking stage, the arm holding surfaces and AST is responsible for validation),
>    intra-file diagnostic order = stage topological order (E3020 → typecheck → W1006/W1003/dead
>    code → proof); diagnostic set unchanged (C2 set semantics), only stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: Make "field is produced but no one consumes it" a fact assertable at compile time
or test time, rather than undetectable. RFC-039 has listed this as "the highest-value item".

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

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                                | Disposition                                                                   |
| ------------------------------------ | -------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Obligation read by declared consumer | Field in `Obligations` is `take()`-ed or marked consumed | Pass                                                                          |
| Obligation non-empty but no consumer | Field non-empty and not consumed                         | **Produce diagnostic** (W level, default; `strict` mode escalates to E level) |
| Obligation empty                     | Field is empty                                           | Pass (no consumption needed)                                                  |

**Why W-level first, not E-level**: Fixing obligation dropping will **change the diagnostic set**.
C2 requires "same diagnostic set per entry"; jumping to E-level in one step would suddenly surface a
lot of previously-silent `Unproven` diagnostics in `yaoxiang check`. Splitting into two steps (W
observation first, then escalate to E) allows each step's criterion to be usable. See implementation
point S4.

**The call site of `assert_drained()` is unique**: `Driver::run` at the end of the stage table,
before producing `CompilationResult`.

**Supporting static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (a field
appearing ≥2 times but no third file reading → fail). `assert_drained()` is the **runtime** gate;
this script is the **static** gate; they complement each other; both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: This note records the
> implementation form of the above sketch and the three-tier table, the measured numbers and the
> 4.3.2 interface, corresponding to code file:line one by one. The landing commit is `1941cc47a`
> (verification tree `99079bc52`). It is also the **third** of the "not silent" three places in the
> red list (the other two: audit surface, static gate).
>
> **1. Landings (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (`:201`, field
>   name/consumer stage/red list), **non-obligation field registration `NON_OBLIGATION_FIELDS`
>   (`:150`, 8 entries with reasons)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), outbound `emit_obligation_diagnostics` (`:654`), non-consumer
>   gate `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped` (`:549`),
>   IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: At the end of `Driver::run`'s stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-exit path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; removing any one will trigger runtime W1090):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only external read path of
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—four same delivery
>   points: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct read of request table) / `:1221`
>   (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation formula
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by use cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, with reason written in the entry above `:26-31`—the settlement granularity
>   is "unit × obligation", without a user-pointable source location, so no dummy span is stuffed);
>   six language texts see `locales/*.json`. **Emission level**: from 4.3.2 onwards it is **error**
>   (same code escalated to E, see paragraph 3)—determined by the explicit
>   `.severity(Severity::Error)` of `to_diagnostic` (obligations.rs:444); the code number is still
>   in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Three-tier table's cell-by-cell landing + two added tiers**
>
> | Three-tier table situation           | Implementation criterion                                                                      | Code                   |
> | ------------------------------------ | --------------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by declared consumer | **Delivery marker exists** (marker ↔ real read is reconciled by static gate, see paragraph 5) | obligations.rs:618-623 |
> | Obligation non-empty but no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see paragraph 3)     | obligations.rs:624-630 |
> | Obligation empty                     | `Obligation::is_produced` checks field-by-field emptiness → pass                              | obligations.rs:587-591 |
>
> Two **added tiers** (2026-10-13 user ruling, not in sketch):
>
> 1. **S1**: The obligation's consumer stage is not in this `ProgramKind`'s stage table → **don't
>    judge as owed** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: Consumer stage skipped with `SkipReason::UpstreamFailed` / `ConfigDisabled`
>    → don't judge as owed (obligations.rs:598-602). `NoObligations` does **not** exempt—an
>    obligation that is non-empty while the stage claims no obligation itself is exactly a ledger
>    imbalance and must be reported.
>
> **3. Outbound standard (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: This mechanism only emits **W level** (W1090); **no strict
> switch**. **4.3.2 (final state, 2026-10-13 user ruling A)**: **Same code escalated to E**—the
> `W1090` code number is unchanged, emission level raised to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code** (one fact two
> codes is parallel representation). **Escalation only changes the emission level, not the emission
> conditions**. **This is D59's D-class breaking change (changes externally observable behavior),
> honestly recorded**: healthy program zero emission (no report when all obligations have delivery
> markers), so existing corpus and user programs are unaffected (342/342 "diff: empty"); but once a
> run actually hits an undelivered obligation, the diagnostic **counts in `error_count`, renders
> `error[W1090]`, and exit code is non-zero**—different from the previous "W level doesn't block
> compilation". The decoupling of code family and emission level has been registered in RFC-013
> "Emission Channel" section (code number prefix only determines number attribution, level is
> determined by explicit severity at construction site). Mechanism facts (measured): severity takes
> explicit `.severity()`, otherwise "code starts with `W` → Warning" (`codes/builder.rs:186-203`),
> the registered `ErrorCategory` does **not** participate in severity; exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`), **W1090 goes to `error_count` due to explicit error (from 4.3.2)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C;
> current red list is already zeroed, see paragraph 4), the reason is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is populated by
>   typecheck's pre-populated table, **constantly non-empty**, the per-compile outbound is a
>   **constant signal** (no discrimination), but it breaks three existing external contracts—"clean
>   program zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing use cases turn red**, of which **8 in
>   out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 in
>   `src/driver/tests/`), corpus baseline 24/342 fixtures change, and the only viable fix is "filter
>   W1090 out of zero-diagnostic assertions" = relaxing existing criteria.
> - Ruling B (downgrade to Info level) still breaks 9/12 of the list-type assertions, and Info is
>   not "output by ruling level".
> - Ruling C's measured: existing use cases **zero changes**, corpus baseline **zero changes**
>   (342/342 "diff: empty, behaviorally equivalent"), runtime W1090 becomes a **discriminating
>   regression detector**—healthy tree silent throughout, only sounds when a delivery marker is
>   removed (three delivery points were each variant-tested, CLI `check` prints 2 `warning [W1090]`
>   when Check arm has marker removed).
>
> The red list's "not silent" is borne by three places: ① `DriverOutcome.obligations.findings` audit
> surface (carried every run, including WBS id and evidence pointer); ② `check-obligations.py`
> prints red list every run (currently empty) + staleness detection; ③ this note.
>
> **4. Red list current state (cleanup and closing) and landing records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three cleanup tasks landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen generates
>   CreateVariant") is equivalently fulfilled by IR-side in-place form detection:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST form
>   `FieldAccess{Call(Var)/Var}` + taking declaration order from `sum_type_variants`), the call site
>   `:7496` replaces the entire Call with `CreateVariant`; the function's doc explicitly writes
>   "span key across systems is unreliable (the two ends have different semantics for Call node
>   span), so detect in-place"—**wiring up the consumer would bring back the already-fixed span
>   mismatch defect**. Deletion surface: `types.rs` field, collection at
>   `checker.rs:1489-1493/:1565`, collection chain in `inference/expressions.rs` and
>   `inference/statements.rs`, `VariantCtorCall` struct in `environment.rs`; type inference and all
>   diagnostics of `infer_variant_ctor_call` **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (in-phase candidate name table
>   and `overload_resolutions` retained). All read/write sites: written in-phase
>   (`environment.rs:205-219` registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550` resolution point, the only hit at `:4550` writes
>   `overload_resolutions`); deleted is the **zero-reader mirror** that `checker.rs:1558` pours into
>   `TypeCheckResult`. Naming facts still expressed at a single point: candidate name table
>   (in-phase) + span-keyed resolution (cross-stage, `ir_gen.rs:380` constructed, `:7632/:7705`
>   consumed); mixed-name on definition side is self-computed by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as in-tree fact** (user ruling A, see the
>   reclassification record in this document's §Obligation Fields table). Deleted is the
>   **declaration** "mirrored for middle/LSP consumption" from the doc comment (types.rs:47-48);
>   retained are the field and its in-phase consumption (`operator_interfaces.rs:623` operator
>   query, seven constraint-solution queries in `inference/expressions.rs`, `checker.rs:214-217`
>   read-only accessor); cross-stage zero reads direct proof
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up the consumer**:
>   the table's content is an environment mirror populated by typecheck's pre-populated table
>   (constantly non-empty), if LSP really needs it, register under 4.6.x; creating a consumer for a
>   field that "by design doesn't need cross-stage consumption" only introduces false obligations.
>
> After the red list is zeroed, **the field survey is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reasons)"**—the gate's criterion 1 reverse
> containment holds this end-to-end (add field and forget to register / delete field and forget to
> sync both turn red immediately). The red list **mechanism** is retained: the gate's criterion 4
> staleness detection and the `--selftest`'s `stale-red-list` fixture continue to cover its
> detection surface; future re-registration must go through explicit ruling.
>
> Synchronized closing standard: obligation list 16 → **13** (`INVENTORY` / `Obligation::ALL` / this
> document's §Obligation Fields table); gate's `--selftest` now covers seven types of fixtures
> (added "unregistered field", "stale anchor", and "red list empty" three forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Static gate (9.2) actual criteria** (three more hard criteria than the one-line statement in
> the sketch)
>
> 1. **Three-place reconciliation + reverse containment (F2) + anchor validation (F1)**:
>    `obligations.rs`'s `INVENTORY` ↔ `TypeCheckResult` field definition ↔ this document's
>    "§Obligation Fields" table (including the header row count declaration), any drift is red; and
>    **`TypeCheckResult`'s field set must be exactly equal to** "obligation list ∪ non-obligation
>    registration (`NON_OBLIGATION_FIELDS`, with reasons)"—add field and forget to register, delete
>    field and forget to sync, non-obligation registration without reason, two tables overlap all
>    turn red immediately;
> 2. **Read-out evidence**: Non-red-list obligations must have a real read (`.field_name` member
>    access; comments/tests/strings don't count) in the **declared consumer file**—this makes
>    "delivery marker ↔ downstream real read" a hard criterion (mechanism A's main risk is precisely
>    the drift between the two);
> 3. **Delivery marker**: Non-red-list obligations must be marked at delivery points; red-list ones
>    must not be marked;
> 4. **Red list staleness detection**: Red list fields must have zero real reads (a consumer growing
>    out is red), and each entry has a WBS id and evidence pointer; plus `--selftest`: built-in
>    negative/positive fixtures, negative must be red, positive must be green (proving the detector
>    is not idle).
>
> **6. Per-place deviations from the above sketch**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **didn't copy per field**: obligation data
>    is still held by `TypeCheckResult`, the ledger only records "list + delivery markers +
>    settlement conclusion". Reason: copying creates two sources of fact (each field is produced by
>    typecheck's checker and ownership layer respectively), they will inevitably drift; the
>    cross-layer data surface also shouldn't be copied to the orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, replaced with `is_produced` emptiness
>    check + `DeliveryLedger` delivery marker. Reason: obligation and product are the same data,
>    `take()` will take away what downstream needs; and the 11 IrGeneration obligations are in the
>    form of "the entire `&TypeCheckResult` handed to ir_gen", which cannot be `take`d field by
>    field.
> 3. The sketch's signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusion written into `DriverOutcome.obligations`,
>    diagnostics split by `ProgramKind`: Check/Lsp goes to per-file collection channel; other forms
>    split by **level** (from 4.3.2 W1090 is E level)—single-file goes to `PipelineError::TypeCheck`
>    (obligations.rs:654 outbound arm), multi-file/embedded separately records `OrchestratorError`
>    fault (otherwise `result.errors` has no consumer). Reason: returning `Vec<Diagnostic>` loses
>    form splitting—check's diagnostics must be per-file.
> 4. Three-tier table's first tier criterion changed from "obligation read by declared consumer" to
>    "has delivery marker"; "marker ↔ real read" is handed to the static gate (paragraph 5 criterion
>    2).
> 5. Three-tier table's second tier's "`strict` mode escalates to E level" **not via strict
>    switch**, changed to **4.3.2 same code escalated to E** (escalated even without strict switch,
>    because healthy state zero emission is already safe enough, R4).
> 6. Static gate criterion upgraded from "field appears ≥2 times but no third file reading" to the
>    four in paragraph 5.
> 7. The two added tiers in the sketch (S1, skip exemption) are in paragraph 2.
> 8. The test name mentioned in S1/S4 tables `test_obligations_drained`: actually landed as **20 use
>    cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turn green; red
>    state retention: after removing the sole settlement point `pairs: 0`, 6 fail). S1/S4 rows have
>    been synchronized to the actual file.
>
> **7. 4.3.2 landing record (level setting + gate hardening)**
>
> - **Field-by-field review input**: `ObligationSettlement.drained_kinds` (the obligation kinds
>   actually drained in this run, available per run) and five counts (paragraph 2 conservation
>   formula); combined with the audit surface `findings`, can give the fact "is this field consumed
>   by any entry".
> - **Level: same code escalated to E is landed** (2026-10-13 user ruling A; see paragraph 3 final
>   state registration and D59 D-class breaking change declaration). Implementation surface:
>   `to_diagnostic` explicit `.severity(Severity::Error)`; outbound arm splits error channel by
>   level. Mutation evidence: remove any one delivery marker → that form's compilation fails,
>   `check` exit code non-zero (`error [W1090]`); restored, healthy state zero emission (corpus
>   342/342 zero change). Locale text has been written in six languages stating "W family code,
>   emission level is error".
> - **Gate hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: The gate now parses **`file:line` anchors in non-obligation
>     registration reasons and red list evidence**, validating "file is locatable (unique) + line
>     number is in range + the symbol immediately after the anchor actually appears in the anchored
>     file"; anchor pointing to a non-existent symbol turns red. Built-in negative fixtures
>     `tests/baselines/obligations-gate/stale-anchor/` (symbol doesn't exist / line number out of
>     range, one each). Note: red list is currently empty, so the actual landing point of this
>     criterion is the **non-obligation registration** and future evidence pointers (same code
>     path).
>   - **F2/R3 reverse containment**: The gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference", non-empty difference turns red;
>     built-in negative fixture `unregistered-field/` (struct adds field without registration).
> - **Info item F3**: `SkipReason::UpstreamFailed` exemption is unreachable in real entries (units
>   with typecheck failure don't retain `type_result`, obligation doesn't exist); this tier
>   semantics is pinned by synthetic `State` use cases, real end-to-end only covers `ConfigDisabled`
>   (t3 T06).
>
> **8. 4.3 secondary task closing (red list zeroed + obligation diagnostic final state E level)**
>
> **8.1 Three-round landing timeline** (one by one corresponding to code file:line; this round only
> adds conclusions, not changes code)
>
> | Round               | Conclusion                                                                                                                                                                            | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
> | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger online | `State::assert_drained()` sole settlement point; 13-entry obligation list + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                        | Call point `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; outbound `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                    |
> | 3.4.4 Cleanup       | `variant_ctor_calls` delete field—declared responsibility equivalently fulfilled by IR-side in-place form detection                                                                   | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                 |
> | 3.4.5 Cleanup       | `method_overload_ir_names` delete cross-stage mirror (in-phase candidate name table + `overload_resolutions` retained)                                                                | Resolution consumption point `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                            |
> | 3.4.9 Cleanup       | `interface_impl_registry` reclassified as **in-tree fact**: delete "mirrored for middle/LSP consumption" declaration, retain in-phase consumption; obligation 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is in the same file `:47-51`, ruling A rewrote); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, `src/frontend/core/typecheck/inference/expressions.rs` seven places (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level setting | **Same code escalated to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; outbound splits error channel by level (first failure priority)                       | `src/driver/obligations.rs:450` (severity), `:676-699` (E level branch); code family and level decoupling registration in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                               |
>
> **8.2 Closing conclusion**
>
> - **Red list zeroed**: Entries in `INVENTORY` (`src/driver/obligations.rs:201`) with
>   `red_list: Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The
>   **mechanism** is retained: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4
>   staleness detection and `stale-red-list` fixture; gate outputs "red list 0 entries—cleared and
>   zeroed".
> - **Obligation diagnostic final state E level**: `W1090`'s **code number and W family attribution
>   unchanged**, emission level is `error` (D5 final state, 2026-10-13 user ruling A). This **is
>   D59's D-class breaking change** (changes externally observable behavior), registered in both
>   this note's paragraph 3 and RFC-013 "Emission Channel" section; **healthy state zero emission ⇒
>   existing corpus and user programs zero impact** (corpus 342/342 "diff: empty", IR snapshot zero
>   drift; t11 independent verification and t12 review each re-verified).
> - **Field survey closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption
>   reason); gate F2 explicitly outputs difference (currently `21 - 13 - 8 = 0`).
> - **No un-landed commitments left**: All hardening items registered in this note are **landed**
>   (F1/F2/F4/F5, see paragraphs 5, 7, 8); the only two not-closed are the **known boundary** in 8.4
>   and t7 F5 which has been turned into an issue, neither of which is an un-landed criterion of
>   this mechanism.
>
> **8.3 Disposition status of each round's acceptance / review findings** (one by one)
>
> | Source        | Finding                                                                                                                                                  | Status                                                                                                                                                                                                                                                          |
> | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red list evidence is plain text pointer, goes silently stale when the substitute consumer is removed                                                     | **Landed**: Gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, silently PASSes when new field is added without registration                                                 | **Landed**: F2 difference explicit output (`:577`) + F4 definition line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                               |
> | t4 R4         | W-only and escalation to E standards not registered                                                                                                      | **Landed**: This note's paragraphs 3/7 + RFC-013 "Emission Channel" + W1090 help and error_output samples in six languages `locales/*.json`                                                                                                                     |
> | t7 F5         | Dead code analysis miscounts overload method references (legal overloads still falsely report `W1005`)                                                   | **Turned into issue `#450`**, attributed to WBS **4.9 reference pool correct implementation** (registered in `docs/src/dev/architecture/09-execution-wbs.md:237`)—not in this secondary task                                                                    |
> | t11 F7        | Under gate's `--root` mode, F1 doesn't cover anchor commitments in module header prose comments                                                          | **Known boundary** (see 8.4)                                                                                                                                                                                                                                    |
> | t11 F8        | Validator-side self-record: mutation drive once wrote to main repo by mistake (immediate `git restore` cleared, affected round output voided and re-run) | **Info item** (not deliverable defect)                                                                                                                                                                                                                          |
> | t11 F9        | Main repo intermediate state (unstaged files make this document's header desync with `INVENTORY`) triggers gate red                                      | **Info item / resolved**: Frozen commit self-consistent PASS; "intermediate state commit directly will turn gate red" is exactly the design intent of F2/F4                                                                                                     |
> | t12 F12-1     | This document's struct range written as `16-73`, actual `16-76` (no mechanical reconciliation after t9 added 3 lines of comments)                        | **Landed + mechanized**: Range corrected to `16-76`, and added gate **F5** (range anchor, red on error); same kind of numbers aligned by grep together (`17 → 20 use cases`, `16 span-keyed fields` → 13)                                                       |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was covered by W1090)                                                | **Fixed** (captain has amended the commit info): `src/driver/obligations.rs:686`'s `failure.is_none()` guard; this regression was caught by the **existing nailed** use case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registration items, not un-landed commitments)**
>
> - **F1's scan surface**: Anchor validation only covers **in-registration entries'**
>   text—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red list evidence;
>   **`file:line` commitments in module header prose comments are not in the scan surface**
>   (currently no in-registration entries affected). If anchor commitments are written in module
>   headers in the future, **manual review must take over**, or expand F1's scan surface—take one of
>   the two, don't default to no owner.
> - **Gate's static boundary**: Set-level validation of delivery markers is on the static side
>   (criterion 3), single-arm removal is supplemented by **runtime settlement**—static + runtime
>   double layer is the design form, not a gap (t11's M11A/M11B/M11C mutation bidirectional
>   verification).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization
> landing)**: Of the above 13 obligations, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) per RFC-039
> **D20**. This note records the landing form, criteria, classification basis, and known boundaries;
> definition lines are still reconciled by gate **F4** (the line numbers in the above two rows are
> current code facts).
>
> **1. Landing form (carrier takes A: `PlanId`)**
>
> The key is a **structural identity** that follows the AST node, no longer relying on the
> positional coincidence of "each side calculates span separately": `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` representing synthetic
> nodes outside the parser) is issued by the **sole allocator** `ParserState::alloc_plan_id`.
> Carriers in two places: `Stmt.plan_id` (statement identity → release plan attribution and
> consumption) and `Expr::Call.plan_id` (call-site identity → overload resolution production and
> consumption); `Expr::call` is the only call expression constructor in the whole repo, synthetic
> calls outside the parser are recorded as `UNPLANNED` (**cannot be consumed**, see paragraph 2).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the unique
>   consumption entry `ReleasePlan::consume(stmt)`—**read and accounting in the same call**;
>   `ir_gen.rs:360` reads, `:3103` uses (statement identity is given by `walk_stmt`'s set/restore +
>   CFG node→statement identity table; scope-exit Drop belongs to the last statement of the scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one body**),
>   production point `inference/expressions.rs:4550` (writes by call-site `PlanId`, dispatch
>   deconstructs from `Expr::Call { plan_id }` for transparent transmission), consumption points
>   `ir_gen.rs:7697` (with binding) / `:7772` (without binding) via `OverloadResolutions::get` (hit
>   and account in one).
>
> **2. Consumption-reaches-settlement (reuse 4.3 ledger, don't create a second set)**: The
> consumption ledger is a **single implementation** `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), each of the two tables holds an instance; when `generate_module_ir`
> successfully returns, it **marks both ledgers' "completed one round" at the same point**
> (`ir_gen.rs:1095` release plan / `:1097` overload resolution); the three-state mapping is unique
> (`consumption_state` in `src/driver/obligations.rs`), and the sole settlement point connects one
> line for each of the two obligations—non-empty difference is **E level W1090** (same code
> escalated to E, see paragraph 3 for standard). Exemption criterion is hooked to the **ledger's own
> completion marker** (not whether `unit.ir` exists), so `check` form taking the IR product doesn't
> constitute an exemption.
>
> **3. D59 classification basis (C class, red then green)**: The
> `Type.method(positional_arg, name = value)` form before the change **silently gives the wrong
> answer** (measured: `r: Int = t.pick(1, a = 2)` compiles cleanly, runtime output `1` instead of
> `20`—the resolved candidate is discarded, falls back to the last bound in definition order); after
> the change, the same program compile-time `error [W1090] ... produced key not consumed`. The fix
> takes **loud** rather than "stuff the resolved name into the call with named arguments": the
> latter would lose named arguments, equivalent to switching to another silence (final state
> semantics in [09](./09-execution-wbs.md) §3.14 / issue #452). Also: `Drop` instructions are no-ops
> in the interpreter (`src/backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`), so the key migration of release plan doesn't change runtime behavior—corpus 342
> entries with empty diff is empirical evidence.
>
> **4. Criteria and evidence**: D41 same-form criterion `test_release_plan_spans_consumed` (settles
> by `PlanId`) and the new `test_overload_resolutions_consumed` (corpus per-file
> lex→parse→typecheck→ir_gen, **difference must be empty + evidence surface floor**: files that
> produced resolutions `≥ 1`, real consumed entries `≥ 2`, no whitelist); resolution form coverage
> of 9 nailed items in `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return
> / positional argument / type body binding / `?` operand / pure named / positional+named /
> ambiguous / variant constructor / unallocated identity). Mutation drive: consumer key offset by
> one ⇒ criterion red + real program `error [W1090]` + corpus gate diff; restored, turns green
> again. Gate numbers: `check-obligations.py` PASS, corpus 342 entries diff empty,
> `cargo test --lib` 2712 passed / 0 failed (IR snapshot zero drift).
>
> **5. Known boundaries (registration items, not un-landed commitments)**: The **no-binding
> consumption point** at `ir_gen.rs:7766-7790` is **unreachable in current corpus and test surface**
> (method declarations uniformly go through `register_type_bindings` to the with-binding consumption
> point), so the keying correctness of that branch has **no red state to falsify**; the reason to
> retain is that it's the symmetric branch of the with-binding path, removing it would rewrite the
> "method call without `type_bindings`" form into silence. Registered by independent verification
> (t5 F-1) and review (t6 F2), **not opening a new task**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project（lib.rs:154）
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project（LSP in-project, results filtered to target file）
    Embedded,          // compile_embedded_module（embedded std）
    WasmPlayground,    // wasm/src/lib.rs:48 —— fourth caller of single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`), not
accepting free arrays from the caller**. This is the key constraint preventing the `Program`
abstraction from degrading into "anything-can-be-passed free parameters", enforced by the
`test_program_stage_coverage` assertion.

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
                // … 12 arms, not one can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the above sketch, semantically equivalent, registered for future reference—
>
> 1. §4's `StageOutcome` three-state is not returned by the stage arm, but expressed by `State`
>    failure marker + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saves
>    boilerplate return from each arm);
> 2. `Driver` has no `config` field—configuration's sole source is `Program.config` (§5 "Avoid
>    Driver holding mutable global state" judgment covers the sketch field);
> 3. `Skipped` diagnostic in 4.1.3 is only recorded in the `DriverOutcome.skipped` internal ledger,
>    not emitted—S2 "deliberately not fixing any bug" zero-diff criterion requires; emission follows
>    4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: driver arm is the only new caller
>    (L1→L2 is allowed direction); orchestrator existing call sites are discussed after 4.2
>    migration.
> 5. Under multi-file form, ProofExecution is an independent stage, running after **all** typecheck
>    (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof in the per-file
>    typecheck loop (file N's proof before file N+1's typecheck). Single failure both are
>    byte-by-byte identical; for the "file 1 proof fails + file 2 typecheck fails" multi-failure
>    scenario, the first report changes from proof error to typecheck error (test nailed).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files are empty Vec—check_project current contract);
>    Discovery product extended to "file set + used_by edge set + shadowing event" triple into State
>    (MultiFile form has no downstream consumer, only records without emitting); Check path per-file
>    parse reduced from 2 to 1 time with C3 topology.
> 8. Lsp form landing (4.2.3, split by file source ruling, 2026-10-09 user decision): Disk file
>    parse failure same as Check's plan B (collect + exit compilation unit—two ends degrade and
>    converge); edited buffer parse failure retains incomplete AST to continue typecheck (editor
>    philosophy—mid-typing state shouldn't make semantic features disappear). Discovery arm retains
>    buffer source overwriting stale disk content for Lsp; Typecheck under Lsp only checks the
>    target file (other units are only for registry signature provision); intra-file diagnostic
>    order normalization principle extends to LSP (typecheck diagnostic → W code warnings → proof
>    error). The old behavior of LSP hard-aborting on unrelated disk file parse errors and handler
>    silently falling back to single-file path is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` adds `shared_registry` field—embedded std module is a
>    sub-compilation, the registry is **input** not product (EMBEDDED_STAGES has no Registry stage),
>    #94's SymbolTable sharing contract is thereby made explicit from "caller remembers to pass the
>    same registry" to program declaration. Error path text normalized from
>    `<std.test> (embedded std)` to unit virtual path `<std/test>` (only compiler internal error
>    surface, no test pinned). At this point all four orchestrator entries migrate into Driver.
> 10. standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): Check form for
>     program without project root is the faithful carrier of single-file semantics—warning surface
>     only covers the entry file (adjacent files only collect errors, better to miss than to be
>     wrong); relative `use` resolves along importer directory (rustc single-file mod alignment).
>     CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full
>     chain) already runs IR generation, E3019/E1014/E1015 etc. only produced in ir_gen (runner gate
>     empirical). IrGeneration's Check form pure check doesn't consume IR; Script/Bin form
>     distinction expressed by module_key's None/Some (E3023's existing switch, ir_gen.rs:1660).
>     Monomorphization doesn't enter CHECK (only produces resource limit/internal error, zero corpus
>     dependency). **〔2026-10-10 withdrawn per 3.4.8 empirical verification〕**—the "zero corpus
>     dependency" was actually evidence blind spot of corpus having no pathological recursion
>     fixture; E3005 is the only compile-time defense for this class of program. See note #14.
>     Collateral fix for latent defect: Registry arm's repeated harvest registration for embedded
>     std unit would override the native half already merged by with_std() (E1043 false
>     report)—embedded unit now skips duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): Delete `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence, go through
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains incomplete AST to
>     continue typecheck (editor philosophy from 4.2.3 ruling extended; not marking stage failure,
>     otherwise topology skip makes typecheck unreachable); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path from here complements proof
>     (E4018), W code warnings (W1001–W1005, test nailed) and IR-level errors; lexical failure
>     changes to real diagnostic (old synthesized "E0001 lexical error" text disappears with
>     sequence deletion—real diagnostic has precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     two entries had already converged—mod.rs layer is `check_module_inner(ast, env, collect_all)`
>     single bool, checker layer is `check_module_impl(module, collect_all)`; the only fork is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, pass-3 and drain two modes run
>     together, the difference is carried by whether collected_errors is empty; five collection
>     points in statements.rs (function body/use/for/block/while). Verification proved and fixed
>     repeated diagnostic defect: collection point puts the first error in both collected_errors and
>     Err return channel, pass-3's add_error on Err causes same code, same span, same message ×2
>     (nested can be ×3); also annotation validation signature parameter and whole annotation
>     double-visit (E1003/E1103), ownership layer same point double-issue (E2014/E2018) two
>     independent mechanisms. Fix lands at module result boundary: deduplicate by (code, span,
>     message)—diagnostic is positional fact, repetition carries no information; 63 corpus entries
>     remove duplicate copies, run column and exit code zero drift (borrow_conflict_err's
>     "duplicate" is artifact of baseline triple format not including column number—two E2018 are
>     actually legitimate diagnostics at different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver—`Compiler` wrapper layer no longer
>     handles. Error text byte-by-byte aligns with old `CompileError` Display prefix (Parse
>     error:/Type error:/Internal error:), playground UI zero visible difference. WasmPlayground and
>     SingleFile share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step has real
>     producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Empirical refutation of "mono only
>     produces resource limit noise": pathological generic recursion (`f(x)=f([x])`) without mono in
>     multi-file path causes compiler process stack overflow (0xc00000fd—interpreter recurses in
>     Rust layer, no graceful runtime error), mono depth gate is the only compile-time defense;
>     standalone check before 4.2.5 went through pipeline full chain and had mono, the exclusion
>     ruling caused silent coverage regression ("zero corpus dependency" only proves corpus has no
>     pathological fixture). "Type erasure fallback" assumption voided per RFC-033 reflection
>     ruling—`^^List(Int)` needs the real identity of instantiation, monomorphization is
>     reflection's foundation not optional optimization. Four design points: ① typecheck for
>     qualified call (`lib.f(x)`, FieldAccess form) produces instantiation_request, generic_id uses
>     qualified name, same namespace as merged IR (currently only recognizes bare Var,
>     expressions.rs:2455); ② `Stage::ALL` topology changed to Linking before Monomorphization,
>     MULTI_FILE stage table tail adds Monomorphization—consumes merged_ir, aggregates all unit
>     requests (deferred bucket's containing_fn qualified), mono upgrades from single-module pass to
>     whole-program pass; single-file stage table unchanged (no Linking, subsequence property
>     preserved, IR snapshot zero drift); ③ CHECK adds Linking (pure merge—E3020 entry validation
>     stays in RoleClassification, prevents double-report) and Monomorphization (pure check doesn't
>     consume, same as IrGeneration's Check form) two stages; ④ resource protection with
>     whole-program BFS coverage of cross-unit mutual recursion. Monomorphizer body unchanged
>     (consumes ModuleIR + request set contract is exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design landing qualified call
>     instantiation collection: `ExpressionInferrer` injected with module qualified key table (`use`
>     three-arm registration—normal / alias / per-item alias and only SubModule class exports),
>     `callee_generic_name` via `SymbolTable::qualify` concatenation with merged IR same-source
>     qualified name; qualified call first path arity criterion tightened (fall back to second path
>     by signature when parameter table can't be solved, avoiding cross-module same-name function
>     mismatch false report E3018). During implementation empirically found and fixed in the same
>     round **three asymmetric defects** on the mono side: (1) Delete key (generic name set) and
>     rewrite key (containing_fn + name + span triple) granularity mismatch—sites that can't solve
>     arguments degrade to symbol request entering deferred bucket, when container function is
>     non-generic bucket never drains, but original was already deleted by name → dangling call
>     (`list_ops.yx:53` runtime E6006 empirical); fix is restore gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as single fact
>     source to determine coverage, uncovered sites' original preserved. (2) Restore segment
>     `HashSet` iteration order causing function table order non-deterministic (t2 review F1) →
>     change to `Vec` and sort by name, byte-level reproducible product. (3) Non-rewrite surface
>     forms TailCall / MakeClosure originally still query mapping keys (comment claims conservative)
>     → `call_form_is_rewritten` single point judgment, non-rewrite forms all count as uncovered.
>     Three each equipped with nailed test (pre-fix red state empirical discrimination). Corpus diff
>     baseline with the same commit reproduces exactly one line (remove_at frame adds `(int64)`).
>     **Commit strategy note**: when typecheck collection and mono fix are split into two commits,
>     the stashed tree containing only the former triggers `stdlib_docs` red against HEAD existing
>     defect (`list.is_empty([])` same family E6006)—the two halves are each other's green light
>     prerequisite, so combined into one atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closing): (1) **D2 ruling A**—user module export
>     carries `generic_fn_type_params_snapshot()` with `Export.type_params`, isomorphic to std path
>     (`yx_sources.rs:62/108`): declaration name becomes the authoritative source of qualified call
>     monomorphization, signature reverse-derivation downgraded to fallback (merged into R2 scope).
>     (2) **HKT declaration side alive, call side broken** (infer pass type argument E1002 /
>     explicit type argument E1010) registered as WBS 3.5 (3 third-level tasks). (3) **deferred
>     bucket never drains under non-generic container** merged into 3.4.8 ②'s acceptance sub-item:
>     bucket must drain or loudly error, must not silently not specialize. (4) **Closure variable
>     name call runtime E6006** registered as WBS 3.6; semantics (i) support indirect call / (ii)
>     compile-time reject, **ruled (i)**—support indirect call: `CallDyn` mechanism is in place
>     (`ir_gen.rs:7945` closure expression path), what's broken is `Var` callee being downgraded to
>     by-name `Call`, and typecheck originally accepts that program. (5) **Tradeoff criterion
>     solidified**—plan rejection reason can only be based on correctness and readability, "large
>     change surface / new mechanism introduced / high cost" doesn't constitute rejection reason,
>     lands in `coding-rules.md` second part and HOWTO self-check table; tradeoffs requiring user
>     decision are given all four items (situation+evidence / per-item pros and cons / clear
>     recommendation+cost / destructive item naming) per `AGENTS.md` "When requesting user ruling
>     (mandatory)".
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail adds mono;
>     Linking arm **before taking each unit's IR** aggregates whole-program requests and qualifies
>     `containing_fn` (otherwise deferred bucket keys and merged IR are from different sources,
>     bucket never drains). **① closing (D2 ruling A)**: user module export carries declaration-time
>     type parameter snapshot with `Export::type_params`, isomorphic to std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically found and fixed during the process**:
>     (a) **F1**—`qualify_names` only rewrites function body, `ir.init` is never visited, under
>     multi-file function definitions are qualified but module top-level calls are still bare name →
>     runtime E6006 (**non-generic immediately reproduces, unrelated to generics**); fix is init
>     reuses same `rename`/`aliases` and same `rewrite_call_names`. (b) mono side "whole-program
>     unique suffix" heuristic with non-hit silent `continue` → delete heuristic, non-hit changes to
>     **loud E3018** (ambiguous silent mismatch is worse than not parsing). **Regression caught and
>     fixed by independent verification**: t6 found named import `use lib.{pick}` followed by bare
>     name call being falsely rejected by E3018 (IR instruction name is alias qualified name,
>     request name is bare name, aggregation side only qualifies by unit key) → fix is aggregation
>     side **directly calls ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as
>     pub single point, one place production one place consumption), with matrix nailed test
>     covering two forms of named import. **Retention**: init segment `DefId` blank empirically is
>     "consistency blank" not live defect (codegen name fallback resolvable, four types of init
>     calls empirically pass), registered in WBS 3.7.4 to close the "function body goes def, init
>     relies on name" criterion fork.
> 18. init segment DefId criterion fork closing (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     extracts `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure),
>     function body and `ir.init` share the same implementation and the same `resolve` closure;
>     after 208 IR snapshots regenerated only 309 pairs of `(def=_)`→`(def=dN)` changed (other-lines
>     0; full tree init segment 0 remaining `(def=_)`). codegen's DefId main path and name fallback
>     two states translation result and execution value are item-by-item consistent (four types of
>     init calls empirically), corpus baseline zero drift; probe flipped and strengthened per new
>     criterion (identity + exact index + execution value, fallback init coverage turns red).
>     **Errata (2026-10-11)**: the above commit numbers were rewritten by
>     `git pull --rebase`—original b554497b/e60dcc87/b7ac7396 correspond to rebased
>     375789d2/d922a551/38bd4b9e in sequence; the pushed dev history uses new numbers (old numbers
>     are local temporary numbers, didn't enter remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (Design basis of ③④ see #14;
>     landing of ② see #17, this entry is its closing.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate requests + `link_module_irs`), entry validation as a whole left to
>     `RoleClassification`; after `ir_generation_check` generates it keeps `us.ir` (memory only);
>     mono failure in Check enters collection channel, in MultiFile maintains `fail_with`;
>     `into_outcome` first `take_check_diagnostics` then judge failure (structural failure no longer
>     clears already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + t5 cross-form matrix) and one round of review (t6) all pass; corpus
>     diff 340/340 line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing
>     gap found and fixed in the same round**: F1 (rootless multi-file check's request name and IR
>     definition key from different sources → silently not specialize, see WBS 3.4.8 row's landing
>     note)—fix is aggregation side precise reverse-lookup by real definition key (non-heuristic),
>     first version fix is aggregation side precise reverse-lookup by real definition key
>     (`flat_defs`); **same-day errata**: this bridge was deleted in the regression fix, root cause
>     instead resolved by `module_key` one-role-two-use decoupling (`script_mode`), three
>     same-source as `{unit key}.{name}`. The regression (CI docs example gate 258 blocks 10 fail →
>     0 fail) location and fix see WBS 3.4.8 row's landing note. **Retention (existing, not this
>     round's regression)**: E3005 help text doesn't match resource protection semantics; multi-file
>     E3005's span label misposition (HEAD binary reproducible); request collection surface's type
>     parameter position dependency (same surface classified as 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closing (2026-10-12, 809b9b936 +
>     8605a081b + subsequent two commits). **3.9.1 (execution layer guard reachable)**: Interpretive
>     call changed to **explicit
>     stack-driven**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` after pushing frame
>     returns control to outer `step_one`, frame carries `ReturnTarget{Host|CallerSlot|Discard}`, on
>     return writes back to caller slot, multiple call instructions use frame-internal
>     `pending_calls` to continue. Baseline measured each level of interpretive call consumes native
>     stack **2544 B**, crash critical at `count_down(394)` (Windows main thread 1 MiB); after fix
>     20,000 levels of recursion can run on 512 KiB stack thread (each level ~0). Guard
>     (`push_frame`) thereby reachable at any entry, any `max_stack_depth` value;
>     `ExecutorError::StackOverflow` changed to carry real limit (previously
>     `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (deep stack backtrace truncation)
>     merged into this task per user ruling** (same file, indivisible by file): backtrace changed to
>     first 16 frames + count ellipsis + last 8 frames, shallow stack frame-by-frame unchanged.
>     **3.10.1 (compile-time ungated native recursion, #447)**: Locate first then change—crash point
>     is `MonoType::type_name` recursion (**328 B/level**, SP probe + per-operation critical
>     determined), same path also has derived `Clone` (~400 B/level), deep copy inside `substitute`,
>     `type_name_hash` (164 B/level), `collect_generic_type_refs`; after four places change work
>     list and remove per-level full request deep copy, `mono.max_depth=5000`'s pathological generic
>     recursion produces **E3005 (with real limit)**, no more stack overflow. **3.11.1 (task
>     boundary error code fidelity, #448)**: Task result error side changed from value to
>     **structured** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), across boundary restores `ExecutorError` by value;
>     intra-task E6001/E6003/E6004/E6008 in Standard mode and Embedded message are
>     character-by-character consistent. Two **pub API payload form changes** have been written in
>     the commit `BREAKING CHANGE`. **Commit strategy note**: 3.9.1 and 3.11.1 are **indivisible at
>     file granularity**—`interpreter/executor/executor.rs` simultaneously carries the trampoline
>     and error channel production/consumption points, `TaskFailure` defined in `runtime/engine.rs`,
>     only stashing either side fails to compile (pre-commit with **stashed subset** compilation,
>     measured twice rejected), so combined into **one atomic commit** per #15's same standard.
>     **Known boundaries and follow-up**: Drop derive (~75 B/level) ⇒ `mono.max_depth ≳ 13000` may
>     still crash; E4012 (constant evaluation) gate carried on native recursion (~3,696 B/level,
>     default 256 levels ≈ 924 KiB), currently zero call site, production unreachable, future wiring
>     must close in same family; Standard mode task interpreter constructed by default
>     `ExecutorConfig` ⇒ its E6004 limit is always 1024 (configuration propagation registered as WBS
>     3.12, user ruling B: separate team); test runtime budget separately opened as issue #449
>     (pathological fixture 76s, clippy dual target, commit runs full), not in this RFC construction
>     surface. **Corpus directory stratification (same round, 8605a081b)**: `02-type-system` (78)
>     and `06-compile-errors` (134) total 212 fixtures split into 23 category-divided
>     subdirectories, 88 IR snapshots relocated by mirror path; corpus baseline with same commit
>     regenerated (342 lines, independent recalculation line-by-line comparison: only `file` field
>     and 2 new changes), 41 snapshots regenerated due to file header path length change,
>     line-by-line verification **only span offset change** (after de-offset normalization zero
>     difference). **Refactoring approach for the ten entries (function by function)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                                |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already stateless wrapper)                                                                                                                                                                            |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                    |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                      |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter result to target file                                                                                                                                                         |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                        |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                           |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                                  |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changed to `Program { kind: Lsp, aggregation: CollectAll }`; **delete** the manual lex→parse→… sequence in single-file branch, change to call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                    |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% forked)**.
Shared: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
parsing loop, per-file checker assembly, result processing. Forked: items 2/3/4/5/7/8/9/10/11 in the
11-item table above. **This 40% shared skeleton is exactly `Driver`'s value range**—the forked 60%
is all "different stage sets" or "different aggregation modes", and these are exactly what the two
fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causing this stage to be skipped" is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped` itself, then the information "why I didn't run" would be scattered across 12 stages,
unable to be centrally audited. Changed to `dispatch` judging by dependency at the start of the
loop:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the bug this document aims to
fix—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip", producing no
diagnostic, simply because the obligation happens to be empty. After the rule is established, any
"because X wasn't done so Y didn't run" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                    | Text Direction                                                                                                  |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                                | "Stage Y not executed due to failure of upstream stage X"                                                       |
| Caused by prerequisite obligation being empty             | "Stage Y not executed due to no pending obligations (normal)"—**this severity = Info, not counted in warnings** |
| Conditions not met (e.g., `config.mono.enabled == false`) | "Stage Y not executed due to configuration not enabled"                                                         |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` relies on this count's non-blocking contract).

### 5. Diagnostic Aggregation Mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                            | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)              | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of "early
exit or not"—they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter, rather than keeping two functions (**this is an
additional item this document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| New `driver/` module                        | **None**. No new type representation introduced, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                              |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only attribution changes. `release_plan` still `ownership::ReleasePlan`, `proof_calls` still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                       |

**This document does not introduce a fourth type representation.** The convergence of three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Attribution and layering of obligation types**: What migrates into the ledger is **settlement
responsibility**, not the attribution module of the type. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the other field types stay where
they are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3
consumers (e.g., `ir_gen` reading `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`**—this is consistent with the driver's
non-reversible-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse
`use crate::driver`**. `Driver` appearing in `TypeChecker`'s import is considered a violation,
intercepted by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Refactoring                       | After Refactoring                                                                                                                                                                                                                                                                                                                       |
| ----------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                           |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already connected in P3 stop-gap; **warning surface maintained without adding** (Decision B1, 2026-10-09: dead code family warnings don't enter compile path—pool semantics is being fixed correctly by 4.9, normalize warning surface left to after 4.9; the initial draft's "add warning output" in this row is void) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already connected in P3 stop-gap                                                                                                                                                                                                                                                                                                        |
| LSP (in-project)                    | No proof_execution                       | Already connected in P3 stop-gap                                                                                                                                                                                                                                                                                                        |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                              |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                                  |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return
diagnostic. This aligns with RFC-027 §8's positioning of "not binding to a specific solver", and is
also the contract already declared in `backend.rs:60` ("caller should conservatively degrade")—the
current implementation is the opposite of its own contract documentation.

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

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                              |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` and served as `Stage::ProofExecution` implementation                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to go through Driver                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` two-choice changed to be driven by `Aggregation` parameter                                                                                                                                                                                                               |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                           |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                 |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                     |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently via `Compiler::compile_with_source` implicitly falling to path 1)                                                                                                                                                                                    |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent drop point**)                                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "lower layer failure prevents upper layer from running" (no short-circuit exists)                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: Add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access point; `predicate.rs`/`checker.rs`/`ownership.rs` three consumer points change to shared access. Bare `&'static` reference form is not feasible (`dyn Solver` is not `Sync`), original expression subject to D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse sequence
at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct—**the problem is on the consumer side, not the producer side**, changing the
producer would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]`
branches (reachability determination of wasm branches goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                   | Compatibility                                                            | Disposition                                                                                                                                          |
| -------------------------------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                     | **Break**: previously silently-passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                   | **Break**: programs that previously compiled now produce warnings        | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), exit code doesn't change                                |
| `assert_drained()` first run produces W level diagnostic | **Break**: diagnostic set adds                                           | First W then E, S2/S4 two steps                                                                                                                      |
| `predicate.rs` panic changed to diagnostic               | **Improvement**: no longer crashes                                       | No break                                                                                                                                             |
| `build` / `dump_bytecode` subcommands                    | **No impact**                                                            | These two paths don't go through proof_execution                                                                                                     |
| `TypeCheckResult` field migration to `Obligations`       | **Internal refactoring**                                                 | If `pub` API surface has external dependencies, need to sync; in-repo consumers are listed in change list                                            |

---

## Implementation Points

This document corresponds to **P3 (fix correctness bugs)** and **P4 (stage contract and unified
Driver)** in RFC-039's global stage sequence. The following S1-S5 are the implementation order
within this document, **each step's acceptance criteria reference the C2 category of the
[equivalence criteria document](07-equivalence-oracle.md)** (orchestration changes: same
**diagnostic set** per entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** The bug criteria must be written as failing first.

| Deliverable                                                                                                                                             | Acceptance                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                           | **Must be red**. If accidentally green, it means the bug analysis of this document needs re-review         |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Decision D48—don't mix into single-file corpus tree; project fixture with `yaoxiang.toml`) | Single-file and multi-file two versions of corpus can compare diagnostic sets                              |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                          | Mark `#[ignore]`, S4 turn green (red state retention: after removing the sole settlement point `pairs: 0`) |

**Rollback point**: No code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-by-byte identical**                     |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately does not fix any bug**—it only moves existing behavior into Driver. The
acceptance criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` can be restored to original.

### S3: Merge Orchestrator Four Entries + Fix Bugs

| Deliverable                                                                                                                    | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructor | Four entry diagnostic sets normalized by `Aggregation` are the same               |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                      | `yaoxiang check` behaves consistently inside and outside project                  |
| LSP manual stage sequence deleted                                                                                              | LSP and CLI give same diagnostic set for same file                                |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                  | Bug fixed                                                                         |
| `checker.rs:1313` empty match arm fixed                                                                                        | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce diagnostic |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                         | No longer panics in no-Z3 environment                                             |

**Rollback point**: Bug fix and structural merge are **two separate commits**. If structural merge
has issues, can only revert the structural commit, retain the bug fix commit—at this time
`test_multifile_proof_obligation_not_dropped` stays green; the reverse (retain structure, revert
fix) will turn red, is an unacceptable intermediate state, prohibited from merging.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                          |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 use cases turn green (implementation form see §2 revision note; count is subject to that file) |
| `scripts/ci/check-obligations.py`            | Static gate in effect                                                                                                               |
| Obligation diagnostic W → E escalation       | Manual review of each new E after full corpus diff                                                                                  |

**Rollback point**: `assert_drained()`'s severity can be controlled by configuration item, W/E
switch doesn't require code structure change.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                            | Acceptance                                                                                                                                                                      |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                 | Document and `checker.rs` call sites correspond one by one                                                                                                                      |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown not in cache**—otherwise a timeout via process-level cache solidifies across compilations and pollutes cargo test inter-thread sharing) | Production path `default_solver()` call sites reduced to zero (only tests retain); cache hit rate observable across three consumer points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                        | Diagnostic present when no `body_checker`                                                                                                                                       |
| Reachability determination of 12 wasm attributes in `orchestrator.rs`                                                                                                                                                  | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination need                                                                               |

**Note**: Correcting the layer order will change the diagnostic set, possibly exposing a lot of
previously-silent `Unproven`. **This item should go independently of S1-S4**, don't mix with entry
merging in the same PR.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                               | Reason                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage Model**          | `Stage` is a 12-variant compile-time exhaustive enum, **runtime registration rejected**                                                                                                                                                                                                                                                                                                                                                     | Exhaustive `match` is zero-cost compile-time enforcement: when adding a variant, `dispatch` fails to compile. This is isomorphic to the mechanism proven effective in the opcode table in RFC-039 routing table B. `StageScope` further makes "per module or project level" a type fact, eliminating the call-count issues that need human reasoning                                                                                 |
| **Obligation Mechanism** | `Obligations` + `assert_drained()` does **runtime** settlement, used alongside the **static** gate of `scripts/ci/check-obligations.py`; **severity final state E level** (2026-10-13 user ruling A: same code escalated to E, code number unchanged; **implementation form see §2 revision note**: ledger doesn't copy field structure; red list—after cleanup 0 entries—mechanism retained, doesn't enter user-visible diagnostic stream) | What we're fixing is a whole class of bugs, not one bug: currently verified same-class hidden dangers at least 2 places (`proof_calls`, `checker.rs:1313`), all 13 obligation fields are in range. W→E two steps is to make each step's C2 criterion usable—now obligation list has been cleaned to 13 entries, red list zeroed, healthy state zero emission, so final state directly takes E (see §2 revision note paragraphs 7, 8) |
| **Fix Scope**            | Fix **consumer side** (orchestration layer), **don't touch** the producer-side logic of `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branches                                                                                                                                                                                                                                                                                  | The problem is on the consumer side, not the producer side. Changing the producer would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                                                                                    |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment commitment" and "code behavior"**. Both `checker.rs:5165`'s "no
  resurrection of silent pass" and `layers/README.md:3`'s "lower layer failure prevents upper layer
  from running" are **comment-level contracts**—the former is fulfilled by `assert_drained()`, the
  latter is exposed by topology-driven `Skipped` diagnostics.
- **"Forget to append in an array"**. Currently five functions each manually write the call
  sequence; after unification, it's a unique stage table, and new stage wiring misses will report
  errors at compile time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—Of the 11 inconsistencies, 7
  (dead code analysis two implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type interface**; it's a configuration question of "whether to
  run some analysis"; it also can't express `Aggregation`, only moves the `check_module` /
  `check_module_collect_all` two-choice from function layer to type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—Removes compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, equivalent to
  changing "five functions each manually write calls" to "forget to append in an array", which is
  exactly the cause of the current 11 inconsistencies. This repo has same-form counterexamples: the
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implementation.
- **Only fix bugs without changing structure**—Can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` is still scattered across 5 functions, and the next fork point will
  continue to grow from here. **It must be done first** (it's part of S1/S3), because structural
  refactoring needs a known red test to prove the criterion is effective.
- **Make `proof_calls` `pub` and add `debug_assert`**—`check_module` is a general entry, it doesn't
  know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]` only
  warns when the field is **entirely** discarded. This is looking for the bug at the wrong level:
  the bug is at the orchestration layer, detection must be at the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs` slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silent problems"**. After the fix, a batch of "new error" user reports may appear—they
  are real bugs that just weren't being reported. Must be explicitly stated in CHANGELOG.
- **`assert_drained()` severity switch needs field-by-field manual judgment**. For some obligation
  fields (like `module_namespaces`), "no consumer" might be by design not needing consumption,
  shouldn't be alerted. S4 needs to go through field by field, can't be one-size-fits-all.
- **`Program` abstraction might be premature**. If some entry's stage set is long-term unstable,
  `stages()` will degrade to "pass different arrays each call" free parameter, contract constraint
  fails. **Mitigation is `test_program_stage_coverage` asserting `stages()` can only come from 6
  predefined combinations.**
- **Criterion dependency**: S1/S3's bug fix criterion depends on
  [equivalence criteria document](07-equivalence-oracle.md)'s bug test being red first. **If that
  document hasn't first established the multi-file corpus layer, S1 cannot be accepted**—because the
  existing 293 corpus all go through single-file path, with zero coverage for this class of defect.
  This document doesn't involve IR verifier (`verify_loose`), that pre-work is not in this
  document's scope.
- **Layer order correction will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order is reversed from the declaration). This is
  the diagnostic set change risk, should go independently.
- **Thread safety after SMT backend becomes singleton is undetermined**. `backend.rs:13-19` already
  states `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after becoming
  cross-compilation-unit shared singleton, need to confirm all access paths go through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (like the actual order dependency of
  termination and ownership). Should be opened after the equivalence criteria are stable.

> **All open questions originally listed in this section have been ruled on.** See each decision in
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactoring (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` bug criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  `build.rs:19-55` gate's reference code table
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "no resurrection of
  silent pass"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "lower layer failure
  prevents upper layer from running" (no short-circuit in reality)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard-fail
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test's own
  statement of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus's own statement of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Fourth compilation caller of the playground (single-file path, so executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate not main crate
- `build.rs:19-55` — Error code build-time gate, the project's example of enforcement mechanism
