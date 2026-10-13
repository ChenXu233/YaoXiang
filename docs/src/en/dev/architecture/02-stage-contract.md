# Compilation Stage Contract and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion grading, and execution stage sequence are in the main text
> of RFC-039; the positioning of each subsidiary document is found in
> [the index of this directory](index.md).

## Positioning and Scope

This document is the **L1 Orchestration Layer** construction blueprint for RFC-039. It addresses one
problem: **"Which stages did this compilation run, which didn't, and why" is currently scattered
across six paths and ten entry functions, each manually wired, with nowhere to answer this
question.**

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are violated by the architecture, not the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` states verbatim "Unproven → compilation
error, no degradation, no silent pass…… must not let silent pass revive". But this invariant only
holds on the **single-file path**: proof function constraints of the form `x: Sorted(3)` **silently
pass** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths, the proof function is
never executed, and no diagnostics are produced.

### Coverage

- The `Stage` enum (exhaustive, non-runtime registration) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry-function transformation approach for
  all ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction norms, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 provides the **why** for refactoring and **in what order** to do it; this document provides
the **concrete form** of L1, **per-file change list**, and the **internal implementation stages** of
this document (corresponding to P3 "Fix Correctness Vulnerabilities" and P4 "Stage Contract and
Unified Driver" in RFC-039's global sequence). In case of conflict between this document and
RFC-039, RFC-039 prevails.

Equivalence criteria are executed under the **C2 (Orchestration Changes)** category of the
[Equivalence Criteria Document](07-equivalence-oracle.md): same diagnostic set per entry + same
corpus behavior.

## Current State

> **This section consists entirely of verified facts**, each with a file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries Are the Only "One Mistake, Silent Whole Repo" Structural Defect

It differs in nature from the other two defect types (module boundaries, test wiring):

| Defect                  | Typical Manifestation         | Has Signal?          |
| ----------------------- | ----------------------------- | -------------------- |
| Lexical/syntactic error | Source code written wrong     | Has diagnostic       |
| Type mismatch           | Type written wrong            | Has diagnostic       |
| Stage missed in wiring  | A stage was never called      | **No signal at all** |
| Obligation not consumed | Field filled but no one reads | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"This project treats 'design' as a documentation convention rather than an executable constraint".

### Mandatory Mechanisms Already in This Repository

The same repository already has mature mandatory mechanisms, they just haven't been extended to the
stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` go through a **build-time
  hard gate** via `build.rs:19-55` and `tools/code-tables`, comparing each entry against the RFC-013
  code table; any inconsistency directly `panic!`s and refuses to compile.
- `src/package/` (**76 files / 13,012 lines**, of which 6,227 lines / 47.9% are in the `tests/`
  directory) has high test density, with each module's test subtree declared and wired—this is the
  most completely test-wired part of the repo and can serve as a formal reference for stage-layer
  gates.

**Design capability is sufficient. What's missing is "placing an equivalent-level gate at the
orchestration layer as well".**

(Line count basis: `(Get-Content).Count`, see the "Line Count Basis" section in
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                 |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                              |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                  |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking            |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics       |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR          |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → Path 1                 |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (Path 3a)                     |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes to 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                     |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → Path 1          |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below provides evidence cell by cell. **A blank does not mean that the entry "doesn't do
this thing", but rather "no code in that entry does this thing"**—this is precisely the nature of
the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                    | **No**                                                      | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                    | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                |
| 4   | W1003 unused import                                      | Yes (collects `type_result.warnings` at `277`)      | **Collects but never outputs**                            | Yes `350`                                                   | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (independent ModuleIR then merge) |

> **Review notes (WBS 3.4.3, per-cell verified 2026-10-07)**: This table is a diagnostic snapshot of
> the `9e02e4db` baseline, preserved unchanged. Differences in current state after P3 lands:
>
> - **Row 1 fixed**: proof_execution's five entries share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + orchestrator four entries), the sole consumer defect
>   no longer exists.
> - **Rows 2/4/5 unchanged** (multi-file `run` still does not output W1003, does not run dead code
>   family) → attributed to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file exclusive) → attributed to WBS 3.4.8 (prerequisite
>   4.1.3, potential risk unproven defect).
> - **Row 7** `check_module` / `check_module_collect_all` dual entry → attributed to 4.2.7
>   (Aggregation parameter-driven).
> - The "#434 ruling" referenced in the code was previously unregistered in docs—has been registered
>   as RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new off-table fact, registered 2026-10-09)**: The
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch); multi-file `discover` previously only recognized
>   explicit `use`—in-project for-loop compiles successfully but emits E6006 at runtime (probe
>   evidence). Fixed and ticked off WBS 4.10.2; the same-family "multi-file lacks one step of
>   single-file" parse hard-abort quirk registered as WBS 4.10.1—**also fixed** (landed right after
>   4.2.2; the Check path degraded to per-file collection; from this audit's perspective of "stage
>   coverage / field consumption" dimension, this is a compilation unit member difference, an
>   overlooked dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   The Registry arm re-harvests registration for embedded std units via `extract_module_info`,
>   which would overwrite the native half already registered via "native + yx surface merge" in
>   `with_std()` (`result.is_err` etc. lost → std.test falsely reports E1043). This audit's third
>   overlooked dimension: "merge semantics of two registration sources for the same module key"—the
>   single-file path's registry is shaped only once via `with_std()`; multi-file/Check's Registry
>   arm inserts per-unit to have coverage.
>
> - **Registry surface coverage of embedded std (discovered during 4.2.5 construction, fixed)**: The
>   Registry arm re-harvests registration for embedded std units using `extract_module_info`, which
>   overwrites the native half already registered via `with_std()` ("native + yx surface merge"),
>   causing `result.is_err` etc. to be lost → std.test falsely reports E1043. This audit's third
>   overlooked dimension: "merge semantics of two registration sources for the same module key"—the
>   single-file path's registry is shaped only once via `with_std()`; the multi-file/Check Registry
>   arm inserts per-unit to gain coverage.

**Two places need precise statement, otherwise errors will occur:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult` (including
  `warnings`), but this function **has no `result.warnings` read site throughout**, and the only
  downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **The `main` criterion for row 11's entries also has different sources**: `check_project:385-395`
  uses `surfaces.bins` (the manifest-declared surface); `compile_project` uses `is_bin_role`
  (`250-252`, only checks "is there a manifest"). The two functions give different answers to "which
  files must define `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The single point where obligations are produced.**
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

The semantics are "the predicate argument of this refinement constraint is a compile-time literal,
so the proof function must actually be executed to make a judgment".

**Step 2 — The consumption point is inside the checker, but it only emits, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original text of the comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `This branch therefore becomes a true line of defense rather than a forward-looking bottoming: must not let silent pass revive.`

That is: the author knew "not producing diagnostics" is a defect, explicitly classified it as a
**known, homeless intermediate state**, and assumed by design that "someone will definitely come to
read `proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects, `checker.rs:1439` writes it to `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read point in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code is **read in only one place, `src/frontend/pipeline.rs:187`**
(passed as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the repository fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` is producer-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — The four orchestrator entries do not pass through that layer.** The `compile_project`
(`99`), `check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module`
(`1374`) in `src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**, but directly call
`TypeChecker::check_module` (`124` / `493` / `1386`). Therefore they cannot even reach that sole
read point at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations also go through the
discard path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling embedded std. This means **the proof obligations of the embedded std module itself are
likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) on multi-file / check / LSP paths **compiles successfully,
runs successfully, with no diagnostics whatsoever**. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent discard point within the layer.**
`checker.rs:1303-1314` when handling the ownership layer's result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` at the ownership check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the "must not let silent pass revive"
promise of the `5164` branch, and is **independent of the orchestrator problem**—even if the entry
layer is completely fixed, this point remains silent. (Processing timing: should be handled in the
same batch as the obligations ledger, because they are the same class of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** The multi-file path has zero coverage of proof obligations.

The three unit tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping exactly before `pipeline.rs`**.

The test file's own doc comment already states this. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases at the .yx level`

**This is exactly the shape of the problem: the test verifies "the obligation is filled", and the
bug is "the consumer doesn't read it".** A test that only tests the producer and not the consumer is
naturally immune to this class of defects.

### A Stronger Finding: The Main Corpus of the Equivalence Criteria Is the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) uses the end-to-end differential of
the 293 `.yx` corpus files in `tests/yaoxiang/` as the **third-layer criterion**, the primary
acceptance tool for the C2 stage. But in practice:

- There is **no `yaoxiang.toml` anywhere in the `tests/` directory** (zero hits in whole-directory
  glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  all do not cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is
empirical evidence of this point. The file is annotated with `// expect: compile-error E4018` (line
15), and the comment at line 7 says "Status: ❌ Should be rejected at compile time"—**it passes,
precisely because it takes the only path that actually executes the proof function**.

**Conclusion: The third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation
Essentials" S1.

### Obligation Fields: 13 "Produce-equals-Contract" Fields, Zero Mechanism Guarantees

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, **13 fields** have doc
comments explicitly stating "produced by stage X → consumed by stage Y"—that is, they are
**cross-stage obligations** in nature:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line number convention**: Definition line = the **declaration line** of that field in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of the doc comment—the comment block for the
> same field may be multi-line, and the declaration line is the only uniquely anchorable position.
> The **struct range** mentioned in the sentence above this table
> (`src/frontend/core/typecheck/types.rs:16-76`) floats with the struct's line numbers and is
> mechanically reconciled by gate **F5** (any error is red); the definition line column is
> reconciled by **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | | `release_plan`
> | `38` | ownership → IR generation (read at `ir_gen.rs:341`, used at `:1942`) | | `escaped_refs` |
> `40` | ownership → IR generation | | `instantiation_requests` | `42` | typecheck →
> monomorphization (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR
> generation | | `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` |
> typecheck → IR generation | | `sum_type_param_names` | `57` | typecheck → IR generation | |
> `operator_dispatches` | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` |
> `62` | typecheck → IR generation (span-keyed) | | `try_expr_impls` | `65` | typecheck → IR
> generation (span-keyed) | | `match_scrutinee_types` | `69` | typecheck → IR generation
> (span-keyed) | | `module_namespaces` | `72` | typecheck → IR generation |

**Ruling record (3.4.9, user ruling A on 2026-10-13)**: The original row 14
`interface_impl_registry` (`types.rs:49`) has been **reclassified as in-tree fact**—by design it is
consumed at phases by typecheck, with zero cross-stage reads, and is not an obligation:

- **Phase consumption points (active)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), seven constraint-solution queries in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), read-only accessor at
  `checker.rs:214-217`, body_checker injection at `checker.rs:974`.
- **Zero cross-stage reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; the `src/driver` side has only the
  ledger's own registration and documentation (removed in this round).
- **Disposition**: Remove the **declaration** in the doc comment saying "mirrored for middle-end/LSP
  consumption"; keep the field and its in-phase consumption; this field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with reason). If LSP genuinely needs this
  table in the future, register it per 4.6.x; **do not create a consumer in this round**.

**All these fields are passed across layers keyed by `Span` or name tables.** The Equivalence
Criteria Document has already pointed out that mismatches in span-keyed contracts "fail silently,
with no error" (`ReleasePlan` is typical: any inconsistency in how the two sides compute span →
`Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
coincidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`) is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module is on the necessary path of all entries.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **another
  entry implementation**. The four orchestrator entries do not pass through the `pipeline.rs` layer
  at all.

**Verified fact**: There is currently **no mechanism in the entire repository** that guarantees
these 13 fields are consumed. The only "protection" is that `ReleasePlan` happens to ride the IR
generation bus.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Fields in `TypeCheckResult` that are not cross-stage
obligations are registered one by one (field name + reason), complementary to the "§Obligation
Fields" table above:

| Field                     | Why Not a Cross-Stage Obligation (Key Point)                                                                            |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit identity metadata (0 hits in `src/middle`)                                                                         |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produce-equals-contract")                                              |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but doc comment does not declare cross-stage contract ⇒ not collected    |
| `local_var_types`         | Diagnostic rendering use (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—contract not declared                        |
| `semantic_db`             | Consumer is in LSP (`src/lsp/server.rs:493`), not a compilation stage obligation                                        |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`, doc comment does not declare contract                                      |
| `warnings`                | Warning diagnostic channel (same kind as `diagnostics`)                                                                 |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption points see "Ruling record" in the "§Obligation Fields" section) |

**Why not "wire up a consumer"**: Creating a downstream for these fields would introduce fake
obligations out of thin air, disguising "by design not requiring cross-stage consumption" as a
contract defect; the correct closure mechanism is **reverse containment**—the `TypeCheckResult`
field set = obligation list ∪ non-obligation registration (judged by
`scripts/ci/check-obligations.py` criterion 1 as a hard rule: add field and forget to register,
remove field and forget to sync, non-obligation registration missing reason, overlap between the two
tables all go red immediately).

### Four Additional Contract Defects in the Proof Layer

The following all come from proof-layer analysis, independent of the entry-fork problem, but all
belong to the category of "stage contracts not enforced".

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

Line 3 of the README claims "executed in layer order, the upper layer does not run when the lower
layer fails". The actual call points in `TypeChecker::check_module`:

| Actual Order | Call Point                                        | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** from declaration; the declared
Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all** (it is only
used by `inference/assignment.rs:17` as the `is_subtype` utility function, unrelated to
`ProofResult`). At the same time, **there is no short-circuit at all**—`checker.rs:1271-1276`
continues to walk down after `add_error`-ing termination's errors one by one, so the README's
promise of "upper layer does not run when the lower layer fails" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-fail panic has been eliminated by P3's 3.3.1 (SOLVER slot
> Option-ified, missing degrades conservatively to `SMTResult::Unknown`); "silent skip (no
> injection)" supplemented by 3.4.2 with W1081 signal. The unification of the three forms
> (singleton-ization) is executed per **RFC-039 D58**, with the actual fix location at
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure access
> port); the `default_solver() → &'static` wording in the 02 change list is subject to D58 (the
> `&'static` bare reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                                   | Acquisition Strategy                                     | When Solver Is Unavailable                                |
| ---------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via injection at `checker.rs:1265-1268`) | Injection at construction `with_solver_owned`            | `None` → **silently skip** (no injection)                 |
| `ownership.rs:627` (back-edge cutoff determination)        | **`default_solver()` every call**                        | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chooses hard-fail: "Initialization failure maintains
**hard fail**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". And the contract doc at `backend.rs:60` says "`None`: backend unavailable……**caller
should degrade conservatively**". **Two philosophies coexist in the same code, and the hard-fail one
panics instead of returning an error.**

**(c) Each back-edge determination creates a new Z3 context, making the cache effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutoff determination. And
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
new Z3 context. The cache field of `Z3Backend` (`cache: RefCell<HashMap<u64, SMTResult>>` at
`proof/smt/z3_backend.rs:20`) is **instance-internal**, while the doc comment at `z3_backend.rs:17`
claims "SMT query results are cached in `cache`". Not shared across calls ⇒ **this cache never hits
under this call pattern**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifetime strategies.)

**(d) Checker silently degrades + comment inconsistent with reality.** `checker.rs:1283-1287` and
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
table** and runs as usual—ownership analysis degrades to "no conflicts at all", **with no warning**.
(Handling suggestion: should be recorded as a warning-level diagnostic, or at least left a trace in
the obligations ledger.)

The comment at `checker.rs:1244` says the termination check "runs after type checking, before
constraint solving". But actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination
is at `1256`, ownership at `1296`, i.e., what the comment calls "before" is actually "two layers
after"**.

### wasm Current State: Carried by shim crate, 27 files have live branches

The wasm target **has been built and is constructed in CI**, with `cdylib` in an independent shim
crate, not the main crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                               |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                               |
| shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                      |
| Main crate has wasm target dependencies section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                  |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                         |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library pre-built by Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from a fixed URL, degrade to warning on missing)                                                                                                                                                               |
| **27 files** contain the `wasm32` literal         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) only mentioned in documentation                                                                                                                                 |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                       |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs in the main crate `wasm/src/lib.rs` (73 lines) can call on the wasm
target—`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole (these
three all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: The playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Paths That Consume `proof_calls`           | Whether They Execute Proof Functions |
| ------------------------------------------ | ------------------------------------ |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                              |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                              |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                              |
| Multi-file `run` (`compile_project`)       | **No**                               |
| `check` (`check_project`)                  | **No**                               |
| LSP in-project (`check_source_in_project`) | **No**                               |

That is: **The existence of `wasm/` turns "proof_execution has only one consumer" into "it has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
new `WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this
path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario—is attributed to `06-cleanup-inventory.md`. The open question in RFC-039 "build
wasm target, or delete these branches" **is already outdated**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Non-Runtime Registration)

**Core constraint: `Stage` is a compile-time exhaustive enum, runtime registration is prohibited.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer"—this is exactly the mechanism that has proven effective for the
opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                   Project
    Parsing,            // Lexical + syntactic              PerModule
    Registry,           // Module registry construction     Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type check (including embedded proof layers) PerModule
    DeadCodeAnalysis,   // Dead code family analysis        Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation           Project
    IrGeneration,       // AST → ModuleIR                   PerModule
    Monomorphization,   // Monomorphization                 Project
    Linking,            // Cross-module linking / IR merge  Project
}

impl Stage {
    /// All stages. When a new variant is added, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once at
the project level) turn "which stages must run per module, which must run once at the project level"
into a type-level fact. Project-scoped stages are structurally guaranteed to run only once in
`dispatch`—this eliminates the kind of question in current `orchestrator.rs` that requires human
reasoning, like "should `allocate_global_slots` be called per file or not".

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the initial draft are
> corrected per implementation evidence—
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the initial draft's order contradicts
>    the data flow.
> 2. The Check form's stage table is normalized to **dead code before proof** per the `Stage::ALL`
>    topological order: `check_project`'s current state runs proof before dead code, the two have no
>    data dependencies, the diagnostic set is the same (C2 set semantics), only the intra-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck, before
>    proof) already conforms to the ALL order, byte-level acceptance is not affected.
> 3. `Parsing` is moved **before** `Registry`/`RoleClassification` (C3, user ruling on 2026-10-09):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    initial draft's order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; and multi-file parse thus drops from 2 to 1. Accompanying current state
>    registered faithfully: the multi-file path's parse error used to be a hard abort
>    (`build_registry_from`'s `?` propagation, tension with CollectAll semantics registered as WBS
>    4.10.1)—**4.10.1 has been fixed** (2026-10-09 user ruling rust-style collection semantics +
>    Plan B): the Check path's parse failure degrades to per-file diagnostic collection, sick files
>    exit the compilation unit (not in registry, importer reports E5001); the MultiFile path keeps
>    hard abort (under FailFast, a bad file cannot produce IR, which is correct semantics, pinned as
>    long-term valid).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data dependency
>    edges 14→16: `RoleClassification` consumes the used_by edge set from Discovery,
>    `DeadCodeAnalysis` consumes W1006 shadow events from Discovery—the initial draft missed these
>    (these two products were produced by `discover_with_used` in the orchestrator era, discarded by
>    the `discover` wrapper, and were lost in the table without entering); b. The intra-file
>    diagnostic order normalization extends to the complete form of note #2: the E3020 entry check
>    moves into the RoleClassification arm (Check has no Linking stage, the arm holding surfaces and
>    AST is responsible for validation), intra-file diagnostic order = stage topological order
>    (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set
>    semantics), only the stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: Turn "field produced but not consumed by anyone" from undetectable to a fact
assertible at compile time or test time. RFC-039 has already listed this as "the highest-value
item".

```rust
// src/driver/obligations.rs
pub struct Obligations {
    pub proof_calls:             Vec<ProofFunctionCall>,     // types.rs:29
    pub release_plan:            ReleasePlan,                // types.rs:31
    pub escaped_refs:            HashSet<String>,            // types.rs:33
    pub instantiation_requests:  Vec<InstantiationRequest>,  // types.rs:35
    pub existential_coercions:   Vec<ExistentialCoercion>,   // types.rs:37
    pub overload_resolutions:    Vec<(Span, String)>,        // types.rs:56
    pub try_expr_impls:          Vec<(Span, String)>,        // types.rs:59
    // …remaining span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                               | Disposition                                                                  |
| ------------------------------------ | ------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Obligation read by declared consumer | Field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                         |
| Obligation non-empty but no consumer | Field non-empty and not consumed                        | **Produce diagnostic** (W level by default; strict mode promotes to E level) |
| Obligation empty                     | Field is empty                                          | Pass (no consumption needed)                                                 |

**Why W level first, not E level**: Fixing obligation discarding will **change the diagnostic set**.
The C2 criterion requires "same diagnostic set per entry"; if we go straight to E level,
`yaoxiang check` will suddenly add a bunch of previously silent `Unproven` diagnostics. Two steps
(observe at W first, then promote to E) make each step's criteria usable. See "Implementation
Essentials" S4.

**`assert_drained()` has only one call site**: `Driver::run` at the end of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appears ≥2 times but has no third file reading → failure). `assert_drained()` is the **runtime**
gate, and the script is the **static** gate; they are complementary and both needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: This note registers the
> **implementation form, measured numbers and 4.3.2 interface** of the above sketch and three-tier
> table, corresponding item by item to code file:line. Landing commit `1941cc47a` (verification tree
> `99079bc52`). It is also the **third** of the three "do not go silent" responsibilities in the red
> list (the other two: audit surface, static gate).
>
> **I. Landing artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (`:201`, field
>   name/consumer stage/red list), **non-obligation field registration `NON_OBLIGATION_FIELDS`
>   (`:150`, 8 entries with reasons)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), outward emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer gate `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **Single settlement point**: At the end of the `Driver::run` stage table, before
>   `into_outcome()` produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The `Aggregation::FailFast` early-exit path also settles (the ledger must
>   balance).
> - **Seven delivery points** (one per arm; remove any one and runtime W1090 will fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only outward read path of
>   `TypeCheckResult` for this obligation, `types.rs:84`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—four same delivery
>   ports: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct read request table) / `:1221`
>   (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation equation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, reason written above the entry at `:26-31`—the settlement granularity is
>   "unit × obligation", with no user-pointable source location, so no dummy span is stuffed);
>   six-language text in `locales/*.json`. **Emission level**: starting from 4.3.2 it is **error**
>   (same code promoted to E, see paragraph three)—decided by explicit `.severity(Severity::Error)`
>   in `to_diagnostic` (obligations.rs:444), the code number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **II. Per-cell landing of the three-tier table + two added tiers**
>
> | Situation in the three-tier table above | Implementation criterion                                                                            | Code                   |
> | --------------------------------------- | --------------------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by declared consumer    | **Delivery marker exists** (marker ↔ actual read is reconciled by the static gate, see paragraph V) | obligations.rs:618-623 |
> | Obligation non-empty but no consumer    | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see paragraph III)         | obligations.rs:624-630 |
> | Obligation empty                        | `Obligation::is_produced` per-field emptiness check → pass                                          | obligations.rs:587-591 |
>
> Two **added tiers** (2026-10-13 user ruling, not in the sketch):
>
> 1. **S1**: The obligation's consumer stage is not in this `ProgramKind`'s stage table → **not
>    judged as owed** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: The consumer stage is skipped due to `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not judged as owed (obligations.rs:598-602). `NoObligations` does **not**
>    exempt—obligations non-empty while that stage claims no obligations, that itself means the
>    ledger is unbalanced, must report.
>
> **III. Outward emission scope (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: This mechanism only emits at **W level** (W1090); **no strict
> switch**. **4.3.2 (terminal state, 2026-10-13 user ruling A)**: **Same-code promoted to
> E**—`W1090` code number unchanged, emission level raised to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code added** (same
> fact with two codes is parallel representation). **The promotion only changes emission level, not
> emission conditions.** **This is a D-class breaking item of D59 (changes externally observable
> behavior), faithfully registered**: a healthy program emits zero (when all obligations have
> delivery markers, no report), so existing corpus and user programs are zero-impact (342/342 "diff:
> empty"); but once a run actually hits an undelivered obligation, that diagnostic is **counted in
> `error_count`, rendered as `error[W1090]`, exit code non-zero**—different from the previous "W
> level does not block compilation". The decoupling of code family and emission level has been
> registered in the RFC-013 "emission channel" section (code number prefix only determines numbering
> ownership, level is determined by explicit severity at construction point). Mechanism fact
> (measured): severity takes explicit `.severity()`, otherwise "code starts with `W` → Warning"
> (`codes/builder.rs:186-203`); the registered `ErrorCategory` **does not participate** in severity;
> exit code is determined by `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and
> `--deny-warnings` (`src/main.rs:567-568`), **W1090 goes to `error_count` (starting 4.3.2) due to
> explicit error**.
>
> **Red list fields do not enter the user-visible diagnostic stream** (captain ruling C on
> 2026-10-13; current red list is zero, see paragraph IV), the reason is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is filled from
>   typecheck's preset table, **always non-empty**, the per-compile outward emission is a **constant
>   signal** (no discriminative power), but it would break three existing external contracts—"clean
>   program zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing test cases turn red**, of which **8 in
>   out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; another 4 in
>   `src/driver/tests/`), corpus baseline 24/342 fixtures changed, and the only viable fix is
>   "filter W1090 out of zero-diagnostic assertions" = relax existing criteria.
> - Ruling B (downgrade to Info level) still breaks 9/12 list-type assertions, and Info is not
>   "output per ruling level".
> - Ruling C measured: **zero existing test changes**, corpus baseline **zero change** (342/342
>   "diff: empty, behaviorally equivalent"), runtime W1090 becomes a **discriminative regression
>   detector**—healthy tree silent throughout, only fires when any delivery marker is removed (three
>   delivery points each underwent marker-removal mutation, CLI `check` emits 2 `warning [W1090]`
>   when Check arm's marker is removed).
>
> The "do not go silent" of the red list is borne by three places: ①
> `DriverOutcome.obligations.findings` audit surface (carried every run, with WBS id and evidence
> pointer); ② `check-obligations.py` prints the red list every run (currently empty) + staleness
> check; ③ this note.
>
> **IV. Current state of red list (settlement closure) and landing records of 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three settlement tasks landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen generates
>   CreateVariant") is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST shape
>   `FieldAccess{Call(Var)/Var}` + taking declaration order from `sum_type_variants`), the call site
>   at `:7496` replaces the entire Call with `CreateVariant`; the function's doc explicitly states
>   "span key cross-system unreliable (the two sides have different semantics for Call node span),
>   so detect in place"—**wiring up a consumer would bring back the already-fixed span mismatch
>   defect**. Deletion surface: `types.rs` field, collection at `checker.rs:1489-1493/:1565`,
>   collection chains in `inference/expressions.rs` and `inference/statements.rs`, `VariantCtorCall`
>   struct in `environment.rs`; type inference and all diagnostics of `infer_variant_ctor_call`
>   **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (kept in-phase candidate name
>   table and `overload_resolutions`). All read/write points: written in phase
>   (`environment.rs:205-219` registered in same order as `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550` resolution point, the only hit writes to `overload_resolutions` at
>   `:4550`); what was deleted is the **zero-reader mirror** filled at `checker.rs:1558` into
>   `TypeCheckResult`. The naming fact is still expressed in a single point: candidate name table
>   (in phase) + span-keyed resolution (cross-stage, constructed at `ir_gen.rs:380`, consumed at
>   `:7632/:7705`); the definition-side mixed names are self-computed by `method_def_ordinals`
>   (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as in-tree fact** (user ruling A, see the
>   ruling record in the "§Obligation Fields" section of this document). What was deleted is the
>   **declaration** in the doc comment saying "mirrored for middle-end/LSP consumption"
>   (types.rs:47-48); the field and its in-phase consumption are kept (`operator_interfaces.rs:623`
>   operator query, seven constraint-solution queries in `inference/expressions.rs`, read-only
>   accessor at `checker.rs:214-217`); cross-stage zero reads direct evidence
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the content of this table is an environment mirror filled by typecheck's preset table (always
>   non-empty); if LSP truly needs it, follow 4.6.x for new registration; creating a consumer for a
>   "by design not needing cross-stage consumption" field only introduces fake obligations into the
>   ledger.
>
> After the red list is zero, **the field census is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reason)"**—the reverse-containment end of gate
> criterion 1 holds this line end-to-end (add field and forget to register / remove field and forget
> to sync both go red immediately). The red list **mechanism** is preserved: gate criterion 4's
> staleness check and `--selftest`'s `stale-red-list` fixture continue to cover its detection
> surface; future re-registration must follow explicit ruling.
>
> Synchronized scope along with settlement: obligation list 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / the "§Obligation Fields" table in this document); gate `--selftest` now covers
> seven categories of fixtures (newly added "unregistered field", "stale anchor" and "empty red
> list" three forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Static gate (9.2) actual criteria** (three more hard criteria than the sketch's single
> sentence)
>
> 1. **Three-place reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ `TypeCheckResult` field definitions ↔ the "§Obligation Fields" table in
>    this document (including the table header count declaration), any drift goes red; and the field
>    set of `TypeCheckResult` must **equal exactly** "obligation list ∪ non-obligation registration
>    (`NON_OBLIGATION_FIELDS`, with reason)"—add field and forget to register, remove field and
>    forget to sync, non-obligation registration missing reason, overlap between the two tables all
>    go red immediately;
> 2. **Read-out evidence**: Non-red-list obligations must have a real read in the **declared
>    consumer file** (`.fieldname` member access; comments/tests/strings don't count)—this turns
>    "delivery marker ↔ downstream real read" into a hard criterion (the main risk of mechanism A is
>    exactly the drift between the two);
> 3. **Delivery marker**: Non-red-list obligations must be marked at the delivery point; the red
>    list conversely must not be marked;
> 4. **Red list staleness check**: Red list fields must have zero real reads (a consumer appearing
>    goes red), and each entry carries a WBS id and evidence pointer; another `--selftest`: comes
>    with negative/positive fixtures, negatives must go red, positives must go green (proves the
>    detector is not idling).
>
> **VI. Per-place deviation from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy field by field**: the
>    obligation data is still held by `TypeCheckResult`; the ledger only records "list + delivery
>    marker + settlement conclusion". Reason: copying would create two sources of fact (each field
>    is produced separately by typecheck's checker and ownership layer), inevitably drifting;
>    cross-layer data surface shouldn't be copied to the orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, replaced with `is_produced` emptiness
>    check + `DeliveryLedger` delivery marker. Reason: obligation and product are the same data,
>    `take()` would take away what the downstream needs; and the 11 IrGeneration obligations are in
>    the form of "the entire `&TypeCheckResult` given to ir_gen", unable to be taken field by field.
> 3. Sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the conclusion is written to `DriverOutcome.obligations`,
>    diagnostics are routed by `ProgramKind`: Check/Lsp go through the per-file collection channel;
>    other forms are routed by **level** (starting 4.3.2 W1090 is E level)—single-file goes to
>    `PipelineError::TypeCheck` (obligations.rs:654 outward emission arm), multi-file/embedded
>    otherwise records `OrchestratorError` fault (otherwise `result.errors` is consumed by no one).
>    Reason: returning `Vec<Diagnostic>` would lose form routing—check's diagnostics must be
>    attributed per file.
> 4. The first-tier criterion of the three-tier table changes from "obligation read by declared
>    consumer" to "has delivery marker"; "marker ↔ real read" is delegated to the static gate
>    (paragraph V, criterion 2).
> 5. The second-tier's "promote to E in strict mode" **does not go through the strict switch**,
>    instead it is **4.3.2 same-code promoted to E** (promoted even without strict switch, because
>    healthy-state zero emission is safe enough, R4).
> 6. The static gate criteria is upgraded from "field appears ≥2 times but no third file reading" to
>    the four criteria in paragraph V.
> 7. The two added tiers not in the sketch (S1, skip exemption) are in paragraph II.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table: actually landed
>    as **20 test cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turn
>    green; red state archive: after removing the single settlement point `pairs: 0`, 6 failures).
>    S1/S4 two rows have been synchronized with the actual file.
>
> **VII. Landing record of 4.3.2 (level + gate hardening)**
>
> - **Per-field review inputs**: `ObligationSettlement.drained_kinds` (kinds of obligations actually
>   drained in this run, available per run) and five counts (the conservation equation in paragraph
>   II); combined with the `findings` audit surface, the fact "whether this field is consumed by any
>   entry" can be given.
> - **Level: same-code promoted to E has landed** (2026-10-13 user ruling A; see the terminal state
>   registration in paragraph III and the D59 D-class breaking item declaration). Implementation
>   surface: `to_diagnostic` explicit `.severity(Severity::Error)`; outward emission arm routes by
>   level to error channel. Mutation evidence: remove any delivery marker → that form's compilation
>   fails, `check` exit code non-zero (`error [W1090]`); after restoration, healthy-state zero
>   emission (corpus 342/342 zero change). Locale text has made it clear in six languages: "W family
>   code, emission level is error".
> - **Gate hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: The gate now resolves `file:line` anchors in **non-obligation
>     registration reasons and red list evidence**, validating "file is locatable (unique) + line
>     number is in range + the symbol immediately following the anchor actually appears in the
>     anchored file"; if the anchor points to a non-existent symbol, it goes red. Comes with
>     negative fixtures `tests/baselines/obligations-gate/stale-anchor/` (symbol non-existent / line
>     out of range one each). Note: the red list is currently empty, so the actual landing point of
>     this criterion is **non-obligation registration** and future evidence pointers (same code
>     path).
>   - **F2/R3 reverse containment**: The gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference", non-empty difference goes red; comes
>     with negative fixture `unregistered-field/` (struct new field unregistered).
> - **Information item F3**: `SkipReason::UpstreamFailed` exemption is unreachable at real entries
>   (units where typecheck fails do not retain `type_result`, the obligation simply does not exist);
>   this tier's semantics is pinned by synthetic `State` test cases, real end-to-end only covers
>   `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 secondary task closure (red list zero + obligation diagnostic terminal E level)**
>
> **8.1 Three-round landing timeline** (each item corresponds to code file:line; this round only
> adds conclusions, does not change code)
>
> | Round               | Conclusion                                                                                                                                                                          | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
> | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger online | `State::assert_drained()` single settlement point; 13-entry obligation list + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                    | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                    |
> | 3.4.4 Settlement    | `variant_ctor_calls` delete field—declared responsibility equivalently fulfilled by in-place shape detection on the IR side                                                         | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                |
> | 3.4.5 Settlement    | `method_overload_ir_names` delete cross-stage mirror (keep in-phase candidate name table + `overload_resolutions`)                                                                  | Resolution consumption point `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                           |
> | 3.4.9 Settlement    | `interface_impl_registry` reclassified as **in-tree fact**: delete "mirrored for middle/LSP consumption" declaration, keep in-phase consumption; obligation 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment in the same file `:47-51`, ruling A rewritten); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, `src/frontend/core/typecheck/inference/expressions.rs` seven places (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level         | **Same-code promoted to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; emission routes by level to error channel (first failure priority)                   | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); registration of decoupling between code family and level see `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                  |
>
> **8.2 Closure conclusion**
>
> - **Red list zero**: Number of entries with `red_list: Some` in `INVENTORY`
>   (`src/driver/obligations.rs:201`) = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). **Mechanism preserved**:
>   `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's staleness check and
>   `stale-red-list` fixture; gate outputs "red list 0 entries—settled to zero".
> - **Obligation diagnostic terminal E level**: The **code number and W family ownership of `W1090`
>   are unchanged**, emission level is `error` (D5 terminal state, 2026-10-13 user ruling A). This
>   **is a D-class breaking item of D59** (changes externally observable behavior),
>   double-registered in paragraph III of this note and the RFC-013 "emission channel" section;
>   **healthy-state zero emission ⇒ existing corpus and user programs zero impact** (corpus 342/342
>   "diff: empty", IR snapshot zero drift; t11 independent verification and t12 review each
>   double-checked).
> - **Field census closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption surface
>   reason); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unlanded commitments**: All hardening items registered in this note have **landed**
>   (F1/F2/F4/F5, see paragraphs V, VII, VIII); the only two items not closed are the **known
>   boundary** in 8.4 and the t7 F5 already converted to issue, neither is an unlanded criterion of
>   this mechanism.
>
> **8.3 Disposition status of various acceptance/review findings** (one by one)
>
> | Source        | Finding                                                                                                                                                       | Status                                                                                                                                                                                                                                                                          |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red list evidence is plain text pointer, goes silently stale when substitute consumer is removed                                                              | **Landed**: gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures `tests/baselines/obligations-gate/stale-anchor/`                                                                               |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, new field unregistered goes silent PASS                                                                           | **Landed**: F2 difference explicit output (`:577`) + F4 definition line reconciliation (`:455`) + F5 range reconciliation; negative fixtures `unregistered-field/`                                                                                                              |
> | t4 R4         | W-only and promoted-E scope not registered                                                                                                                    | **Landed**: paragraphs III/VII of this note + RFC-013 "emission channel" + W1090 help and error_output samples in six languages `locales/*.json`                                                                                                                                |
> | t7 F5         | Dead code analysis miscounts overload method references (legal overload still falsely reports `W1005`)                                                        | **Converted to issue `#450`**, attributed to WBS **4.9 Reference Pool Correct Implementation** (`docs/src/dev/architecture/09-execution-wbs.md:237` registration)—not this secondary task                                                                                       |
> | t11 F7        | In gate `--root` mode, F1 doesn't cover anchor commitments in module header prose comments                                                                    | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                    |
> | t11 F8        | Verification tool self-record: mutation driver once accidentally wrote to main repo (instant `git restore` cleanup, affected rounds' output voided and rerun) | **Information item** (not a delivery defect)                                                                                                                                                                                                                                    |
> | t11 F9        | Main repo intermediate state (unstaged files make this document's table header and `INVENTORY` out of sync) triggered gate red                                | **Information item / resolved**: frozen commit self-consistent PASS; "intermediate state committed directly will red the gate" is exactly the design intent of F2/F4                                                                                                            |
> | t12 F12-1     | Struct range in this document written as `16-73`, actually `16-76` (no mechanical reconciliation after t9 added 3 comment lines)                              | **Landed + mechanized**: range corrected to `16-76`, and added gate **F5** (range anchor, error goes red); same kind of numbers aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                                             |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was overwritten by W1090)                                                 | **Fixed** (captain already amended commit message to explain): `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned** test case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registration items, not unlanded commitments)**
>
> - **F1's scan surface**: Anchor validation only covers text in **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red list evidence;
>   **`file:line` commitments in module header prose comments are not in the scan surface**
>   (currently no registered entry is affected). If in the future `file:line` commitments are
>   written in module headers, must be **borne by manual review**, or extend F1's scan surface—take
>   either, do not allow default no one in charge.
> - **Static boundary of the gate**: Set-level validation of delivery markers is on the static side
>   (criterion 3), per-arm removal is supplemented by **runtime settlement**—static + runtime
>   double-layer is the design form, not a gap (t11's M11A/M11B/M11C mutation bidirectional
>   verification).

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

**`stages()` can only be one of the above 6 predefined combinations (one per `ProgramKind`), does
not accept the caller freely passing an array.** This is the key constraint preventing the `Program`
abstraction from degenerating into "free parameters for anything", enforced by
`test_program_stage_coverage` assertion.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines whether to skip, not determined by stage return value
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
        state.obligations.assert_drained();           // Single settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for reference—
>
> 1. §4's `StageOutcome` three-state is not returned by the stage arm, but expressed by `State`
>    failure flag + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saving
>    per-arm boilerplate return);
> 2. `Driver` has no `config` field—the single source of configuration is `Program.config` (§5
>    "Avoid Driver holding mutable global state" judgment covers the sketch field);
> 3. `Skipped` diagnostic in 4.1.3 is only recorded in `DriverOutcome.skipped` internal ledger, not
>    emitted outward—S2's "deliberately fix no bug" zero-diff criterion requires this; emission goes
>    with 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: driver arm is the only new caller
>    (L1→L2 is allowed direction); orchestrator existing call site is moved out with 4.2 and
>    returned to position later.
> 5. In multi-file form, ProofExecution is an independent stage, running after **all** typechecks
>    (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof in the per-file
>    typecheck loop (file N's proof precedes file N+1's typecheck). Single-failure case both are
>    byte-identical; "file1 proof failure + file2 typecheck failure" multi-failure scenario, first
>    report changes from proof error to typecheck error (test pinned).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (each found file has an entry, clean files have empty Vec—`check_project` current contract);
>    Discovery product extended to "file set + used_by edge set + shadow events" triple enters State
>    (MultiFile form has no downstream consumer, only records not emitted); check path each file
>    parse drops from 2 to 1 with C3 topology.
> 8. Lsp form landing (4.2.3, routing by file source ruling, 2026-10-09 user decision): disk file
>    parse failure same as Check's Plan B (collect + exit compilation unit—two-side degradation
>    convergence); edited buffer parse failure retains incomplete AST to continue typecheck (editor
>    philosophy—mid-typing state shouldn't make semantic functionality disappear). Discovery arm for
>    Lsp keeps buffer source code overwriting disk stale content; Typecheck under Lsp only checks
>    target file (other units only for registry to provide signatures); intra-file diagnostic order
>    normalization principle extends to LSP (typecheck diagnostic → W code warning → proof error).
>    The old behavior where LSP hard-aborts on unrelated disk file parse error, and the handler
>    silently falls back to single-file path quirk is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` adds new `shared_registry` field—embedded std module
>    is sub-compilation, registry is **input** not product (EMBEDDED_STAGES has no Registry stage),
>    #94's SymbolTable sharing contract is from "caller remembers to pass the same registry"
>    explicitized to program declaration. Error path text from `<std.test> (embedded std)`
>    normalized to unit virtual path `<std/test>` (only compiler internal error surface, no test
>    pinned). At this point orchestrator four entries all moved into Driver.
> 10. standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): Check form
>     faithful bearer of single-file semantics for program without project root—warning surface only
>     covers entry file (adjacent files only collect errors, rather miss than false-positive);
>     relative `use` resolves along importer directory (rustc single-file mod alignment). CHECK
>     stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full chain)
>     already runs IR generation, E3019/E1014/E1015 etc. only produced in ir_gen (runner gate
>     evidence). IrGeneration's Check form pure check does not consume IR; Script/Bin split
>     expressed with module_key's None/Some (E3023 existing switch, ir_gen.rs:1660).
>     Monomorphization does not enter CHECK (only produces resource limit/internal errors, zero
>     corpus dependency). **〔Withdrawn by 3.4.8 evidence on 2026-10-10〕**—"zero corpus dependency"
>     is actually evidence blind spot of corpus having no pathological recursion fixture; E3005 is
>     the only compile-time defense line for such programs. See note #14. Additionally fixed latent
>     defect: Registry arm's repeated harvest registration for embedded std units would overwrite
>     the native half already merged by `with_std()` (E1043 false-positive)—embedded unit now skips
>     repeated registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`' manual
>     lex→parse→check_module_collect_all sequence deleted, changed to
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI use the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains incomplete AST to
>     continue typecheck (editor philosophy extended from 4.2.3 ruling; doesn't mark stage failure,
>     otherwise topology skip would make typecheck never reachable); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path from this point on completes
>     proof (E4018), W code warnings (W1001–W1005, test pinned) and IR-level errors; lexical failure
>     changes to report real diagnostic (old synthetic "E0001 lexical error" text disappears with
>     sequence deletion—real diagnostic carries precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries have long converged—mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single bool, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, difference carried by
>     whether collected_errors is empty; statements.rs five collection points (function
>     body/use/for/block/while). Verification confirmed and fixed duplicate diagnostic defect:
>     collection points put the first error both in collected_errors and Err return channel,
>     pass-3's add_error on Err causes same code same span same message ×2 (nested can be ×3); there
>     are also annotation validation signature parameter and overall annotation double visit
>     (E1003/E1103), ownership layer same point double emit (E2014/E2018) two independent
>     mechanisms. Fix landed at module result boundary: deduplicate by (code, span,
>     message)—diagnostic is positional fact, duplicates carry no information; 63 corpus entries
>     removed duplicate copies, run column and exit code zero drift (`borrow_conflict_err`'s
>     "duplicate" is an artifact of baseline triple format not containing column number—two E2018
>     are actually legal diagnostics of different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground`
>     constructing `Program { kind: WasmPlayground }` + Driver—`Compiler` wrapper layer no longer
>     involved. Error text byte-aligned with old `CompileError` Display prefix (Parse error:/Type
>     error:/Internal error:), playground UI zero visible difference. WasmPlayground and SingleFile
>     share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step gets a real
>     producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of mono exclusion in note #10**). Evidence refutes "mono only produces
>     resource limit noise": pathological generic recursion (`f(x)=f([x])`) without mono on the
>     multi-file path causes compiler process stack overflow (0xc00000fd—the interpreter recurses at
>     the Rust layer, no graceful runtime error), mono depth gate is the only compile-time defense
>     line; standalone check before 4.2.5 went through pipeline full chain and had mono, the
>     exclusion ruling caused silent coverage regression ("zero corpus dependency" only proves
>     corpus has no pathological fixture). "Type erasure fallback" assumption voided by RFC-033
>     reflection ruling—`^^List(Int)` needs the real identity of instantiation, monomorphization is
>     the foundation of reflection, not an optional optimization. Four design points: ① typecheck
>     produces instantiation_request for qualified calls (`lib.f(x)`, FieldAccess form), generic_id
>     uses qualified name, sharing namespace with merged IR (currently only recognizes bare Var,
>     expressions.rs:2455); ② `Stage::ALL` topology changed to Linking before Monomorphization,
>     MULTI_FILE stage table tail adds Monomorphization—consume merged_ir, aggregate all unit
>     requests (deferred bucket's containing_fn qualified), mono upgrades from single module pass to
>     whole program pass; single-file stage table unchanged (no Linking, subsequence property
>     preserved, IR snapshot zero drift); ③ CHECK adds Linking (pure merge—E3020 entry check left to
>     RoleClassification, prevent double report) and Monomorphization (pure check no consumption,
>     same as IrGeneration's Check form) two stages; ④ resource protection with whole program BFS
>     covers cross-unit mutual recursion. Monomorphizer body unchanged (consuming ModuleIR + request
>     set contract is exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design, qualified call instantiation
>     collection lands: `ExpressionInferrer` injects module qualified key table (`use` three-arm
>     registration—normal / alias / per-item alias and only SubModule kind export),
>     `callee_generic_name` is concatenated via `SymbolTable::qualify` with the same source
>     qualified name as merged IR; qualified call first path arity criterion tightened (when
>     parameter table can't be solved, fall to second path taking real arguments by signature,
>     avoiding cross-module same-name function mismatching false E3018). Three asymmetric defects on
>     the mono side empirically found and fixed in the same round: (1) Delete key (generic name set)
>     and rewrite key (containing_fn + name + span triple) granularity mismatch—when real arguments
>     can't be solved, the site degrades to symbol request entering deferred bucket, when container
>     function is non-generic bucket never drains, but the original has been deleted by name →
>     dangling call (`list_ops.yx:53` runtime E6006 evidence); fix is restore gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as single source of
>     fact to determine coverage, uncovered site's original kept. (2) Recovery segment `HashSet`
>     iteration order causes function table order non-deterministic (t2 review F1) → changed to
>     `Vec` and sorted by name, product byte-level reproducible. (3) Non-rewrite surface forms
>     TailCall / MakeClosure originally look up mapping key as usual (comment claims conservative) →
>     `call_form_is_rewritten` single point determination, non-rewrite forms always counted as
>     uncovered. Each with pinned test (pre-fix red state evidence discriminative power). Corpus
>     differential baseline regenerates exactly one line in same commit (remove_at frame adds
>     `(int64)`). **Commit strategy note**: when typecheck collection and mono fix are split into
>     two commits, only the former's staged tree triggers `stdlib_docs`'s red on HEAD existing
>     defect (`list.is_empty([])` same-family E6006)—the two halves are each other's green light
>     prerequisite, so merged into one atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **D2 ruling A**—user module export
>     carries `generic_fn_type_params_snapshot()` with `Export.type_params`, same structure as std
>     path (`yx_sources.rs:62/108`): declaration name becomes authoritative source of qualified call
>     monomorphization, signature back-inference degraded to fallback (folded into R2 scope). (2)
>     **HKT declaration side active, call side broken** (inferring type argument E1002 / explicitly
>     giving type argument E1010) registered as WBS 3.5 (3 level-3 tasks). (3) **deferred bucket
>     never drains under non-generic container** folded into acceptance sub-item of 3.4.8 ②: bucket
>     must drain or loudly error, not silently not specialize. (4) **Closure variable by-name call
>     runtime E6006** registered as WBS 3.6; semantic (i) support indirect call / (ii) compile-time
>     reject, **ruled (i)**—support indirect call: `CallDyn` mechanism already in (`ir_gen.rs:7945`
>     closure expression path), broken is `Var` callee being downgraded to by-name `Call`, and
>     typecheck originally accepted this program. (5) **Trade-off criterion solidification**—plan
>     rejection reason can only be based on correctness and readability, "large change surface /
>     introduce new mechanism / high cost" does not constitute rejection reason, fall to
>     `coding-rules.md` second part and HOWTO self-check list; trade-offs needing user decision
>     follow `AGENTS.md` "When requesting user ruling (mandatory)" give all four items (situation +
>     evidence / per-item pros and cons / clear recommendation + cost / breaking items by name).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② per #14 design lands:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE tail adds mono; Linking
>     arm **before taking each unit's IR** aggregates whole program request and qualifies
>     `containing_fn` (otherwise deferred bucket key and merged IR different source, bucket never
>     drains). **① Closure (D2 ruling A)**: user module export carries declaration-phase type
>     parameter snapshot with `Export::type_params`, same structure as std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically found and fixed in process**: (a)
>     **F1**—`qualify_names` only rewrites function body, `ir.init` is never visited, multi-file
>     function definition already qualified but module top-level call still bare name → runtime
>     E6006 (**non-generic reproduces**, unrelated to generics); fix is init reusing same
>     `rename`/`aliases` and same `rewrite_call_names`. (b) mono side "whole program unique suffix"
>     heuristic and miss silent `continue` → delete heuristic, miss change to **loud E3018**
>     (ambiguous silent mismatch worse than not parsing). **Independently verified regression caught
>     and fixed**: t6 found named import `use lib.{pick}` bare-name call was falsely rejected by
>     E3018 (IR instruction name is alias qualified name, request name is bare name, aggregate side
>     only qualifies by unit key) → fix is aggregate side **directly calling ir_gen's**
>     `import_aliases_from` and `resolve_call_target` (extracted to pub single point, one producer
>     one consumer), and matrix pinned test covers named import two forms. **Filed**: init segment
>     `DefId` blank empirically confirmed as "consistency blank" not active defect (codegen name
>     fallback resolvable, four types of init call empirically pass), registered as WBS 3.7.4 to
>     close the "function body goes def, init relies on name" scope fork.
> 18. init segment DefId scope fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operand + MakeClosure), function
>     body and `ir.init` share the same implementation and the same `resolve` closure; 208 IR
>     snapshots regenerated with only 309 `(def=_)`→`(def=dN)` changes (other-lines 0; whole tree
>     init segment 0 residual `(def=_)`). Codegen's DefId main path and name fallback two states'
>     translation result and execution value are item-by-item consistent (four types of init call
>     measured), corpus baseline zero drift; probe flipped and strengthened per new scope
>     (identity + precise index + execution value, fallback init coverage causes red). **Errata
>     (2026-10-11)**: above commit numbers rewritten via `git pull --rebase`—original
>     b554497b/e60dcc87/b7ac7396 correspond to post-rebase 375789d2/d922a551/38bd4b9e in turn; the
>     pushed dev history uses new numbers (old numbers are local temporary, did not enter remote
>     history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④ design basis see #14; ② landing
>     see #17, this entry is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate request + `link_module_irs`), entry check whole block left to `RoleClassification`;
>     `ir_generation_check` after generation keeps `us.ir` (memory only); mono failure in Check goes
>     to collection channel, in MultiFile keeps `fail_with`; `into_outcome` first
>     `take_check_diagnostics` then judge fault (structural fault no longer empties already
>     collected diagnostic). **Verification**: four rounds of independent verification (t3/t4/t10 +
>     t5 cross-form matrix) and one review (t6) all pass; corpus differential 340/340 line-by-line
>     IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gap found and fixed in
>     same round**: F1 (rootless multi-file check's request name and IR definition key different
>     source → silently not specialize, see WBS 3.4.8 row's landing note)—fix is aggregate side by
>     real definition key precise reverse lookup (non-heuristic), first version fix is aggregate
>     side by real definition key precise reverse lookup (`flat_defs`); **same-day errata**: this
>     bridge has been deleted in regression fix, root cause instead solved by `module_key`
>     one-job-two-use decoupling (`script_mode`), three same source as `{unit key}.{name}`. The
>     location and fix of this regression (CI document example gate 258 blocks 10 failures → 0
>     failures) see WBS 3.4.8 row's landing note. **Filed (existing, not this round's regression)**:
>     E3005 help text inconsistent with resource protection semantics; multi-file E3005's span label
>     misalignment (HEAD binary reproducible); type parameter position dependency of request
>     collection surface (same surface attributed to 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + subsequent two commits). **3.9.1 (execution layer guard reachable)**: Interpretive
>     call changed to **explicit
>     stack-driven**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push frame then hand
>     control back to outer `step_one`, frame carries `ReturnTarget{Host|CallerSlot|Discard}`, on
>     return write back to caller slot, multi-call instructions use frame's `pending_calls` to
>     continue. Baseline measured each layer of interpretive call consumes native stack **2544 B**,
>     crash critical at `count_down(394)` (Windows main thread 1 MiB); after fix 20,000 layer
>     recursion can run on 512 KiB stack thread (each layer about 0). Guard (`push_frame`) thus
>     reachable at any entry, any `max_stack_depth` value; `ExecutorError::StackOverflow` changed to
>     carry real limit (original `util/diagnostic/mod.rs` always passes 0). **Original 3.11.2 (deep
>     stack echo truncation) merged into this task by user ruling** (same file, cannot be split by
>     file): echo changed to first 16 frames + count ellipsis + last 8 frames, shallow stack
>     frame-by-frame unchanged. **3.10.1 (compile-time unguarded native recursion, #447)**: locate
>     first then change—crash point is `MonoType::type_name` recursion (**328 B/layer**, SP probe +
>     per-operation critical determined), same path also has derived `Clone` (~400 B/layer),
>     `substitute` inner deep copy, `type_name_hash` (164 B/layer), `collect_generic_type_refs`;
>     after four places changed to work list and removed each layer's one full request deep copy,
>     pathological generic recursion with `mono.max_depth=5000` produces **E3005 (with real
>     limit)**, no longer stack overflow. **3.11.1 (task boundary error code fidelity, #448)**: task
>     result error side changed from value to **structured**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`),
>     cross-boundary by value restore `ExecutorError`; task-internal E6001/E6003/E6004/E6008 in
>     Standard mode and Embedded message verbatim consistent. Two **pub API payload form changes**
>     have `BREAKING CHANGE` in commit. **Commit strategy note**: 3.9.1 and 3.11.1 **cannot be split
>     at file granularity**—`interpreter/executor/executor.rs` simultaneously carries trampoline and
>     error channel's producer/consumer points, `TaskFailure` defined in `runtime/engine.rs`, only
>     staging either side doesn't compile (pre-commit with **staged subset** compile, measured two
>     rejections), so per #15 same scope merged into **one atomic commit**. **Known boundary and
>     follow-up**: Drop derive (~75 B/layer) ⇒ `mono.max_depth ≳ 13000` may still crash; E4012
>     (constant evaluation) gate carried on native recursion (each layer ~3,696 B, default 256 layer
>     ≈ 924 KiB), currently zero call site, production unreachable, future wiring must close in same
>     family; Standard mode task interpreter constructed by default `ExecutorConfig` ⇒ its E6004
>     limit always 1024 (config propagation registered as WBS 3.12, user ruling B: separate team);
>     test runtime budget opened as issue #449 (pathological fixture 76s, clippy dual target, commit
>     runs full), not in this RFC construction scope. **Corpus directory layering (same round,
>     8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures split
>     into 23 subdirectories by category, 88 IR snapshots relocated in mirror path; corpus baseline
>     regenerated in same commit (342 lines, independent recount line-by-line: only `file` field and
>     2 new changes), 41 snapshots regenerated due to file header path getting longer, line-by-line
>     verification **only span offset changes** (after de-offset normalization zero difference).

**Ten entry transformation approach (function by function)**:

| Entry                                                      | After Transformation                                                                                                                                                                                      |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` handed to Driver                                                                                                              |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already stateless wrapper)                                                                                                                                                                     |
| `compile_project` (`orchestrator.rs:99-237`)               | Slimmed to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                               |
| `check_project` (`orchestrator.rs:273-399`)                | Slimmed to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                 |
| `check_source_in_project` (`orchestrator.rs:450`)          | Changed to one Driver call + result filter to target file                                                                                                                                                 |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Changed to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                    |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                           |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changed to `Program { kind: Lsp, aggregation: CollectAll }`; **manual lex→parse→… sequence of single-file branch deleted**, changed to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                             |

**`check_project` and `compile_project` degree of duplication (about 40% shared / 60% fork)**.
Shared: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
resolution loop, per-file checker assembly, result processing. Forked: items 2/3/4/5/7/8/9/10/11 in
the 11-item table above. **This 40% shared skeleton is exactly the value interval of `Driver`**—the
forked 60% is all "different stage set" or "different aggregation mode", and these are exactly what
the two fields of `Program` can completely express.

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

Reason: skip is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped` itself, then the information "why didn't I run" is scattered across 12 stages, unable to
be centrally audited. Changed to `dispatch` judging at the beginning of the loop based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability to be fixed in
this document—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a silent
"skip", it doesn't produce any diagnostic, just because the obligation happens to be empty. After
the rule is established, any "because X wasn't done, so Y wasn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                             |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Upstream `Abort` caused                                  | "Stage Y did not execute because upstream stage X failed"                                                                                  |
| Prerequisite obligation empty caused                     | "Stage Y did not execute because there were no pending obligations (normal)"—**this item's severity = Info, not counted in warning count** |
| Condition not met (e.g., `config.mono.enabled == false`) | "Stage Y did not execute because configuration is not enabled"                                                                             |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)              | Exists                       |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists                       |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program can avoid Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of "whether
to exit early"—they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter, rather than retaining two functions (**this is an
additional verification item needed for this document, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                              |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation introduced, no touching `MonoType` / `PolyType` / `ir::Type`                                              |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` still `ownership::ReleasePlan`, `proof_calls` still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                     |

**This document does not introduce a fourth set of type representations.** The convergence of three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: What is migrated into the ledger is **settlement
responsibility**, not the ownership module of the type. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` still in `proof/`, the rest of the field types stay in
their original places; `driver/obligations.rs` only holds the aggregate container and
`assert_drained()`. L3 consumers (such as `ir_gen` reading `release_plan`) continue to receive
specific field types through parameters, **must not `use crate::driver`**—this is consistent with
the driver's non-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**.
`Driver` appearing in `TypeChecker`'s import is considered a violation, intercepted by the
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                                    |
| ----------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                           |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already stop-bled in P3; **warning surface maintained without new additions** (Decision B1, 2026-10-09: dead code family warnings don't enter compile path—pool semantics being corrected as defect by 4.9, normalizing warning surface deferred until after 4.9; this row's initial draft "new warning output" voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already stop-bled in P3                                                                                                                                                                                                                                                                                                                 |
| LSP (in-project)                    | No proof_execution                       | Already stop-bled in P3                                                                                                                                                                                                                                                                                                                 |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                              |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                                  |

**The only intentional behavior break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver" and is
also the contract already declared by `backend.rs:60` ("caller should degrade conservatively")—the
current implementation contradicts its own contract documentation.

### Compiler Change List

**New Files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file / N for multi-file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified Files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                             |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and serves as `Stage::ProofExecution` implementation                                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to Driver                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or changed to be driven by `Aggregation` parameter                                                                                                                                                                                                                       |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`' manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                            |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently goes through `Compiler::compile_with_source` implicitly falling to path 1)                                                                                                                                                                                  |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent discard point**)                                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "upper layer does not run when lower layer fails" (no short-circuit exists)                                                                                                                                                                                                                |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumer points in `predicate.rs`/`checker.rs`/`ownership.rs` changed to use the shared port. `&'static` bare reference form not feasible (`dyn Solver` is not `Sync`), original wording subject to D58 |

**Deletions**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct—**the problem is on the consumer side, not the producer side**, changing the
producer would mask architectural defects); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]`
branch (reachability determination of wasm branches attributed to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                   | Compatibility                                                          | Disposition                                                                                                                                       |
| -------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                     | **Breakage**: previously silently passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; batched release, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                   | **Breakage**: programs that compiled now produce warnings              | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                             |
| `assert_drained()` first run produces W-level diagnostic | **Breakage**: diagnostic set adds                                      | W first then E, S2/S4 two steps                                                                                                                   |
| `predicate.rs` panic changed to diagnostic               | **Improvement**: no longer crashes                                     | No breakage                                                                                                                                       |
| `build` / `dump_bytecode` subcommands                    | **No impact**                                                          | These two paths don't go through proof_execution                                                                                                  |
| `TypeCheckResult` fields migrated to `Obligations`       | **Internal refactoring**                                               | If `pub` API surface has external dependencies, need to sync; in-repo consumers are fully listed in the change list                               |

---

## Implementation Essentials

This document corresponds to **P3 (Fix Correctness Vulnerabilities)** and **P4 (Stage Contract and
Unified Driver)** in the RFC-039 global stage sequence. S1-S5 below are the implementation order
within this document, **each step's acceptance criterion cites the C2 category of the
[Equivalence Criteria Document](07-equivalence-oracle.md)** (orchestration changes: same
**diagnostic set** per entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written as failing first.

| Delivery                                                                                                                                                 | Acceptance                                                                                                  |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                            | **Must be red**. If accidentally green, the vulnerability analysis in this document needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Decision D48—not mixed with single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file versions comparable                                           |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                           | Mark `#[ignore]`, S4 turn green (red state archive: after removing the single settlement point `pairs: 0`)  |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Delivery                                                 | Acceptance (C2)                                                                               |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equal to expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-identical**                              |
| Full corpus (293 `.yx`) differential                     | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**  |

**This stage deliberately fixes no bug**—it only moves existing behavior into Driver. The acceptance
standard is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restore to original.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Delivery                                                                                                                       | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructor | Four entries' diagnostic set normalized by `Aggregation` is the same              |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                      | `yaoxiang check` behaves consistently inside and outside the project              |
| LSP manual stage sequence deleted                                                                                              | LSP and CLI give the same diagnostic set for the same file                        |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                  | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                        | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                         | No longer panics in Z3 environment                                                |

**Rollback point**: vulnerability fix and structural merge **are two separate commits**. If
structural merge has problems, can only roll back the structural commit, keep the vulnerability fix
commit—at this time `test_multifile_proof_obligation_not_dropped` remains green; reverse (keep
structure, roll back fix) will turn red, which is an unacceptable intermediate state, prohibited
from merging.

### S4: Enable Obligations Ledger

| Delivery                                     | Acceptance                                                                                                                           |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 test cases turn green (implementation form see §2 revision note; case count based on that file) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                             |
| Obligation diagnostic W → E upgrade          | Full corpus differential then manual review each new E                                                                               |

**Rollback point**: `assert_drained()`'s severity can be controlled by a config item, W/E switching
doesn't need to change code structure.

### S5: Proof Layer and wasm Wrap-up

| Delivery                                                                                                                                                                                                             | Acceptance                                                                                                                                                          |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                               | Documentation corresponds item-by-item with `checker.rs` call points                                                                                                |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown not in cache**—otherwise one timeout fixes across compilations via process-level cache, and pollutes cargo test inter-thread sharing) | Production path `default_solver()` call site zeroed (only tests kept); cache hit rate observable across three consumer points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                      | No `body_checker` has diagnostic                                                                                                                                    |
| `orchestrator.rs` 12 wasm attributes reachability determination                                                                                                                                                      | Conclusion handed to `06-cleanup-inventory.md`, this document only registers the determination need                                                                 |

**Note**: Correcting the layer order will change the diagnostic set, possibly exposing a large
number of previously silent `Unproven`. **This item should walk independently of S1-S4**, don't mix
entry merge in the same PR.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                         | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time exhaustive enum with 12 variants, **runtime registration refused**                                                                                                                                                                                                                                                                                                                                          | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant, `dispatch` fails to compile. This is isomorphic to the mechanism verified in RFC-039 routing table B for the opcode table. `StageScope` further turns "per module or project level" into a type fact, eliminating call-count questions that require human reasoning                                                                                                  |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, in parallel with the **static** gate of `scripts/ci/check-obligations.py`; **terminal severity E level** (2026-10-13 user ruling A: same code promoted to E, code number unchanged; **implementation form see §2 revision note**: ledger doesn't copy field structure; red list—after settlement 0 entries—mechanism preserved, not in user-visible diagnostic stream) | What is being fixed is an entire class of bugs, not one: at least 2 places of the same kind of hidden danger have been verified (`proof_calls`, `checker.rs:1313`), all 13 obligation fields are in range. W→E in two steps is to make each step's C2 criterion usable—now the obligation list has been settled to 13 entries, red list zero, healthy state zero emission, so terminal state directly takes E (see §2 revision note paragraphs VII, VIII) |
| **Fix scope**            | Fix **consumer side** (orchestration layer), **do not touch** the six `Unproven` branch producer logic at `checker.rs:5164/5179/5306/5318/5420/5448`                                                                                                                                                                                                                                                                                  | The problem is on the consumer side, not the producer side. Changing the producer would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                                                                                                         |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment commitment" and "code behavior"**. Both "must not let silent pass
  revive" at `checker.rs:5165` and "upper layer does not run when lower layer fails" at
  `layers/README.md:3` are **comment-level contracts**—the former is fulfilled by
  `assert_drained()`, the latter is exposed by topology-driven `Skipped` diagnostics.
- **"One array forgotten to append"**. Currently five functions each manually write the call
  sequence; after unification it's a single stage table, and new stage missed wiring will error at
  compile time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (dead code analysis two implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, are configuration questions of "whether to
  run a certain analysis"; it also can't express `Aggregation`, only moving the either-or of
  `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancels compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, equal to changing
  "five functions each manually write call" to "one array forgotten to append", exactly the cause of
  the current 11 inconsistencies. This repository already has same-form counterexample:
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bug without changing structure**—can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` still scattered in 5 functions, the next fork point will continue to
  grow from here. **It must be done first** (part of S1/S3), because structural transformation needs
  a known red test to prove the criterion is effective.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is looking for bugs at the wrong level:
  the bug is in the orchestration layer, the detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silent problems"**. After the fix, a batch of "new error" user reports may appear—they
  are real bugs, just never reported. Must be clearly explained in CHANGELOG.
- **`assert_drained()`'s severity switching needs per-field manual judgment**. For some obligation
  fields (such as `module_namespaces`), "no consumer" may be by design not needing consumption,
  shouldn't alarm. S4 needs to go through each field, can't be one-size-fits-all.
- **`Program` abstraction may be premature**. If some entries' stage sets are unstable in the long
  term, `stages()` will degrade to "passing a different array each call" free parameter, the
  contract constraint fails. **Mitigation is `test_program_stage_coverage` asserting `stages()` can
  only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criteria depend on the vulnerability test in
  the [Equivalence Criteria Document](07-equivalence-oracle.md) being red first. **If that document
  doesn't first establish the multi-file corpus layer, S1 cannot be accepted**—because the existing
  293 corpus all go through the single-file path, with zero coverage of this class of defect. This
  document doesn't involve the IR validator (`verify_loose`), that part of the prerequisite work is
  not in this document's scope.
- **Layer order correction will expose previously silent `Unproven`** (`equivalence` not in the
  pipeline at all, `termination` and `ownership` order is reversed from declaration). This is a
  diagnostic set change risk, should walk independently.
- **Thread safety after SMT backend becomes singleton is undetermined**. `backend.rs:13-19` already
  states `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after changing to a
  singleton shared across compilation units, need to confirm all access paths go through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization would **mask order-dependency defects** (such as the actual order dependency of
  termination and ownership). Should be opened after the equivalence criteria are stable.

> **The open questions originally listed in this section have all been ruled on.** Item-by-item
> decisions see [RFC-039 Ruling Registration](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction norms,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`' `include!` transformation (construction steps attributed to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Comparison code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "must not let silent
  pass revive"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "upper layer does not
  run when lower layer fails" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard-fail
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 settlement)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Playground's fourth compilation caller (single-file path, so executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate not main crate
- `build.rs:19-55` — Error code build-time gate, example of this project's mandatory mechanism
