# Compilation Stage Contract and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage sequence are in the body of
> RFC-039; the positioning of each subsidiary document is in the [directory index](index.md).

## Positioning and Scope

This document is the **L1 Orchestration Layer** construction drawing for RFC-039. It addresses one
problem: **"which stages ran in this compilation, which didn't, and why"** is currently scattered
across six paths and ten entry functions, each wired up by hand, with no single place able to answer
this question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself have been broken by architecture, not logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` reads verbatim "Unproven → compile
error, no degradation, no silent pass……must not let silent pass come back to life". But this
invariant only holds on the **single-file path**: proof function constraints of the form
`x: Sorted(3)` silently pass under the three paths of multi-file `yaoxiang run`, `yaoxiang check`,
and LSP—the proof functions never execute, and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, no runtime registration) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-entry migration scheme for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1–C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- Global execution order of P1–P10 and G1–G10 acceptance gates → RFC-039 body

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and the **order in which to do it**; this document
gives the L1's **specific form**, **per-file change list**, and the **implementation stages within
this document** (corresponding to P3 "Fix Correctness Vulnerabilities" and P4 "Stage Contracts and
Unified Driver" in RFC-039's global sequence). Where this document conflicts with RFC-039, RFC-039
prevails.

Equivalence criteria are executed under the **C2 (orchestration changes)** category of the
[equivalence criteria document](07-equivalence-oracle.md): same diagnostic set across entry points +
same corpus behavior.

## Current State

> All items in this section are **verified facts**, each with a file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries Are the Only Structural Defect of "Write It Wrong Once, Silent Failure Throughout"

It differs in nature from the other two defect types (module boundaries, test wiring):

| Defect                | Typical Manifestation            | Has Signal?          |
| --------------------- | -------------------------------- | -------------------- |
| Lexical/syntax error  | Source code written wrong        | Has diagnostic       |
| Type mismatch         | Type written wrong               | Has diagnostic       |
| Stage skipped         | Some stage simply not invoked    | **No signal at all** |
| Obligation unconsumed | Field filled but no one reads it | **No signal at all** |

The common characteristic of the latter two is: **failure does not produce an error**. Therefore
they cannot be solved by "writing code more carefully" or "reviewing more strictly"—code review can
only see what was written, not **what was not written**. This is exactly the root cause diagnosed by
RFC-039: "This project treats 'design' as a documentation convention, not as an executable
constraint".

### Existing Enforcement Mechanisms in This Repository

The same repository already has mature enforcement mechanisms—they just haven't been extended to the
stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are enforced by a
  **build-time hard gate** via `build.rs:19-55` through `tools/code-tables`, which compares each one
  against the RFC-013 code table; any inconsistency causes direct `panic!` to refuse compilation.
- `src/package/` (**76 files / 13,012 lines**, of which `tests/` is 6,227 lines / 47.9%) has high
  test density, and each module's test subtree is declared-wired—this is the most complete
  test-wiring area in the entire repository and can serve as a formal reference for stage-layer
  gates.

**The design capability is sufficient. What's missing is "a gate of the same level in the
orchestration layer as well".**

(Line count method: `(Get-Content).Count`, see the "Line Count Method" section in
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                               |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | Direct 5-stage call                             |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking          |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics    |
| 3c  | **LSP in project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR        |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1               |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                   |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In project: path 3c; otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                   |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1        |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)— RFC-027 Phase 2.5                  |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below gives evidence cell by cell. **An empty cell does not mean "this entry doesn't do
this", but "there is no code in this entry that does this"**—this is exactly the nature of the
defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                        | `compile_project`                                      | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------ | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                               | **No**                                                 | **No**                                                       | **No**                             | **No**                                 |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gate) | **No**                                                 | Yes `356-377` (role-aware, no config gate)                   | **No**                             | **No**                                 |
| 3   | W1006 local module shadowing                             | **No**                                          | **No**                                                 | Yes `336-348`                                                | **No**                             | **No**                                 |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)     | **Collected but never output**                         | Yes `350`                                                    | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                 | **No**                                                 | Yes (same as 2)                                              | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gate)      | **No**                                                 | N/A                                                          | N/A                                | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                  | `check_module` (`124`), **first-error return** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                             | `discover` (`101`, discards `used_by`/`shadow_events`) | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                          | **No**                                                 | Yes (`321-328`)                                              | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | N/A                                             | Yes `allocate_global_slots` (`147`)                    | **No**                                                       | **No**                             | **No**                                 |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                     | Yes (`152-236`)                                        | **No**                                                       | **No**                             | Yes (independent ModuleIR then merged) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, retained unchanged. Status differences after P3 lands:
>
> - **Row 1 has been fixed**: the five entries of proof_execution share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the single-consumer defect
>   no longer exists.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still does not output W1003, does not run the
>   dead code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file exclusive) → assigned to WBS 3.4.8 (prerequisite 4.1.3,
>   unproven risk, not a defect).
> - **Row 7** `check_module` / `check_module_collect_all` dual entries → assigned to 4.2.7
>   (Aggregation parameter-driven).
> - The "Ruling #434" referenced in the code side was previously zero-registered in docs—now
>   back-registered as RFC-039 **D57**.
> - **Embedded std into asymmetry (new fact outside the table, 2026-10-09 back-registered)**: The
>   single-file path injects std.list unconditionally via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch), but multi-file `discover` previously only recognized
>   explicit `use`—for loops within projects compiled fine but produced runtime E6006 (probe
>   empirical evidence). Fixed and WBS 4.10.2 crossed off; the same family "multi-file lacks a
>   single-file step" of parse hard-stop quirk is registered as WBS 4.10.1—**also fixed** (landed
>   right after 4.2.2; Check path degraded to per-file collection; from this audit method's "stage
>   coverage / field consumption" perspective, this one is a compilation unit membership difference,
>   a missed dimension).
>
> - **Embedded std registration surface coverage (found during 4.2.5 construction, fixed)**: The
>   Registry arm re-harvests registration for the embedded std unit via `extract_module_info`, which
>   overwrites the native half already registered by `with_std()` in "native + yx surface merge"
>   mode (loss of `result.is_err` etc. → std.test false E1043). This audit's third missed dimension:
>   the merge semantics of "two registration sources for the same module key"—the single-file path's
>   registry is formed only once through `with_std()`, while the Registry arm of multi-file/Check
>   inserts per-unit to have coverage.

**Two places need precise expression, otherwise they will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` under the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; under the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  at `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function has **no `result.warnings` read site throughout**, and
  the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **The `main` criterion for row 11's entry is also not the same source**: `check_project:385-395`
  uses `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`,
  only checks "whether a manifest exists"). The two functions give different answers to "which file
  must define `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The production site of obligations is unique.**
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
this proof function must actually be executed to determine".

**Step 2 — Consumption sites are inside the checker, but only emit, never consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The comment at `checker.rs:5165-5169` reads verbatim:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real line of defense, not an anticipatory fallback: must not let silent pass come back to life.`

That is: the author was fully aware that "producing no diagnostic" is a defect, and explicitly
classified it as a **known, go-nowhere intermediate state**, designed on the assumption that
"someone will definitely come to read `proof_calls`".

**Step 3 — That field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects; `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code is **only read at `src/frontend/pipeline.rs:187`** (passed as
argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other occurrences of the `proof_calls` identifier across the repository fall into three categories,
none of which are consumers of this field: `checker.rs:1234/4564/5494` are production-side
collections; `verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` reads
`ProofResult`.)

**Step 5 — All four entries of the orchestrator bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
of `src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**, and directly call
`TypeChecker::check_module` (`124` / `493` / `1386`). Therefore they cannot even reach that sole
read site at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also take the
discarded path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible
for compiling the embedded std. This means **the proof obligations of the embedded std modules
themselves are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP three paths **compiles, runs,
and produces no diagnostic at all**. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent discard site within the layer.**
`checker.rs:1303-1314` when handling the ownership layer's result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the promise of branch `5164` that "silent
pass must not come back to life", and is **independent of the orchestrator problem**—even if the
entry layer is completely fixed, this one will still remain silent. (Processing timing: should be
handled in the same batch as the obligations ledger, because they are the same class of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of the proof obligation on the multi-file path.

The three unit tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `SumUpTo` calls exist
in `result.proof_calls`. But they **all go via `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping just
before `pipeline.rs`**.

The test file's own doc comment has already written this down.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases live at the .yx layer`

**This is exactly the shape of the problem: tests verify "obligations are filled in", but the bug is
"the consumer doesn't read".** A test that only tests the producer, not the consumer, is naturally
immune to this class of defect.

### Stronger Finding: The Main Corpus of the Equivalence Criteria Is the Single-File Path

The [equivalence criteria document](07-equivalence-oracle.md) takes the end-to-end differential of
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion** and the primary
acceptance means for the C2 stage. But empirically:

- **There is no `yaoxiang.toml` anywhere in the `tests/` directory** (zero hits across the whole
  directory glob).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for each corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is precisely the
empirical evidence of this. The file is annotated `// expect: compile-error E4018` (line 15), and
its header comment line 7 says "Status: ❌ should be rejected at compile time"—**it passes,
precisely because it takes the only path that actually executes the proof function**.

**Conclusion: the third-layer criterion needs a supplementary multi-file corpus layer, otherwise it
cannot serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 16 "Produce-as-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields** whose
doc comments explicitly state "produced by stage X → consumed by stage Y", meaning they are
essentially **cross-stage obligations**:

| Field                      | Definition Line | Producer → Consumer                                             |
| -------------------------- | --------------- | --------------------------------------------------------------- |
| `proof_calls`              | `29`            | typecheck → **proof_execution**                                 |
| `release_plan`             | `31`            | ownership → IR generation (`ir_gen.rs:341` reads, `:1942` uses) |
| `escaped_refs`             | `33`            | ownership → IR generation                                       |
| `instantiation_requests`   | `35`            | typecheck → monomorphization (`pipeline.rs:381`)                |
| `existential_coercions`    | `37`            | typecheck → IR generation                                       |
| `implementation_proofs`    | `39`            | typecheck → IR generation                                       |
| `interface_impl_registry`  | `42`            | typecheck → operator query / constraint solving / LSP           |
| `sum_types`                | `44`            | typecheck → IR generation                                       |
| `sum_type_param_names`     | `47`            | typecheck → IR generation                                       |
| `variant_ctor_calls`       | `49`            | typecheck → IR generation (span-keyed)                          |
| `operator_dispatches`      | `51`            | typecheck → IR generation (span-keyed)                          |
| `method_overload_ir_names` | `53`            | typecheck → IR generation                                       |
| `overload_resolutions`     | `56`            | typecheck → IR generation (span-keyed)                          |
| `try_expr_impls`           | `59`            | typecheck → IR generation (span-keyed)                          |
| `match_scrutinee_types`    | `63`            | typecheck → IR generation (span-keyed)                          |
| `module_namespaces`        | `66`            | typecheck → IR generation                                       |

**All these fields are passed across layers keyed by `Span` or name tables.** The equivalence
criteria document has already pointed out that span-keyed contract mismatches will "silently fail,
with no error at all" (`ReleasePlan` is typical: any inconsistency in how the two sides compute
spans → `Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- `release_plan`'s consumer `ir_gen.rs:341` (`release_plan: type_result.release_plan.drops.clone()`)
  is inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in a downstream module, and the downstream module is on
  the must-pass path of every entry.
- `proof_calls`'s consumer is at `pipeline.rs` **top level**, belonging to **another entry
  implementation**. The orchestrator's four entries never go through the `pipeline.rs` layer.

**Verified fact**: the entire repository currently has **no mechanism whatsoever** to guarantee that
these 16 fields are consumed. The only "protection" is that `ReleasePlan` happens to hitchhike on IR
generation.

### Four Additional Contract Defects in the Proof Layer

All of the following come from the proof layer analysis, are independent of the entry-fork problem,
but all belong to the category "stage contract is not enforced".

**(a) The layer order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "executed in layer order, lower layer failure means upper layers do not run". The
actual call sites in `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** relative to the declaration; the
declared Layer 0 `equivalence` **does not appear at all in the stage sequence of `check_module`**
(it is only used as an `is_subtype` utility function by `inference/assignment.rs:17`, unrelated to
`ProofResult`). And at the same time **no short-circuit exists**—`checker.rs:1271-1276` adds each
termination error via `add_error` and then continues forward; the README's promise of "lower layer
failure means upper layers do not run" does not hold.

**(b) The SMT backend has three acquisition strategies, two failure philosophies.**

> **Review note (2026-10-07)**: The hard-failure panic has been eliminated by 3.3.1 of P3 (SOLVER
> slot Option-ified, missing degrades to `SMTResult::Unknown` conservatively); "silent skip (no
> injection)" is back-filled with the W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is executed per **RFC-039 D58**; the real fix location is
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure entry
> point); the `default_solver() → &'static` expression in the 02 change list follows D58 (the
> `&'static` bare reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Injected at construction via `with_solver_owned`         | `None` → **silently skip** (no injection)                 |
| `ownership.rs:627` (back-edge cut determination)       | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "Initialization failure stays
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". But `backend.rs:60`'s contract documentation says "`None`: backend
unavailable……**caller should degrade conservatively**". **Two philosophies coexist in the same code,
and the hard-failure one panics rather than returning an error.**

**(c) A new Z3 context is created for every back-edge determination; caching is a dead letter.**
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
`cache: RefCell<HashMap<u64, SMTResult>>`) is **per-instance**, and the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**that cache never hits under this call mode**.

(Comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. The same backend, two
lifecycle strategies.)

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
table** and runs as normal—ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Processing suggestion: should be recorded as a warning-level diagnostic, or at least
left a trace in the obligations ledger.)

The comment at `checker.rs:1244` claims termination check "runs after type checking, before
constraint solving". Actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination is
at `1256`, ownership is after `1296`, i.e., the "before" the comment speaks of is actually "after
two layers"**.

### Wasm Current State: Carried by Shim Crate, the 27-File Branch Is Alive

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                           |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate only `rlib`                            | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                 |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                 |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                        |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                    |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                           |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to warning when missing)                                                                                                                                                             |
| **27 files** contain the `wasm32` literal         | Of which **25** carry actual `#[cfg(...)]` attributes; the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in documentation                                                                                                                              |
| `orchestrator.rs` 20 occurrences                  | 12 attribute + 8 comment                                                                                                                                                                                                                                                           |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                         |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call on the wasm
target—`lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` as a whole (these
three all need `std::fs`), and the shim takes a different path.

**But this brings a fact directly related to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path that consumes `proof_calls`           | Executes proof functions? |
| ------------------------------------------ | ------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                   |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                   |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                   |
| Multi-file `run` (`compile_project`)       | **No**                    |
| `check` (`check_project`)                  | **No**                    |
| LSP in project (`check_source_in_project`) | **No**                    |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
new `WasmPlayground` variant (see "Target Design" §3), otherwise the unified Driver will miss this
path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario—belongs to `06-cleanup-inventory.md`. The RFC-039 open question "build the wasm
target, or remove these branches" **is already expired**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, No Runtime Registration)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "a new stage must be
handled by the orchestration layer"—this is exactly the mechanism that the opcode table has proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexing + syntax                 PerModule
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type checking (with embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis       Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Monomorphization,   // Monomorphization                Project
    Linking,            // Cross-module linking / IR merge Project
}

impl Stage {
    /// All stages. When a new variant is added, this array and the exhaustive match in `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at the project level) make "which stages must be run per module, which must be run once at the
project level" a type-level fact. Stages with `Project` scope are structurally guaranteed to run
only once in `dispatch`—this eliminates questions like "should `allocate_global_slots` be called
once per file" that currently require human reasoning in `orchestrator.rs`.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the initial draft are
> corrected per implementation evidence—
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the initial draft order contradicts the
>    data flow.
> 2. The stage table of the Check form is normalized by `Stage::ALL` topological order as dead code
>    **before** proof: the current state of `check_project` runs proof before dead code; the two
>    have no data dependency, the diagnostic set is the same (C2 set semantics), only the in-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck, before
>    proof) already conforms to the ALL order, and byte-level acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    initial draft order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; and multi-file parse therefore drops from 2 times to 1 time. The
>    accompanying honest registration of the current state: the multi-file path's parse error was
>    once a hard stop (the `?` propagation of `build_registry_from`, the tension with CollectAll
>    semantics registered as WBS 4.10.1)—**4.10.1 has been fixed** (2026-10-09 user ruling on
>    rust-style collection semantics + Option B): the Check path's parse failure is degraded to
>    per-file diagnostic collection, and the diseased file exits the compilation unit (does not
>    enter registry, importer reports E5001); the MultiFile path retains hard stop (under FailFast,
>    a bad file cannot produce IR, which is the correct semantics, and the nailed board is valid
>    long-term).
> 4. The Check form landing (4.2.2, 2026-10-09 user ruling) back-registers two items: a. The
>    data-dependency edges go 14→16: `RoleClassification` consumes Discovery's `used_by` edge set,
>    and `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the initial draft omitted
>    these (these two products were produced by `discover_with_used` and discarded by the `discover`
>    wrapper in the orchestrator era; the data flow got lost just by not being in the table); b. The
>    in-file diagnostic order normalization is extended to the full form of note #2 above: E3020
>    entry validation is moved into the RoleClassification arm (the check has no Linking stage; the
>    arm holding surfaces and AST is responsible for validation), and the in-file diagnostic order =
>    stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set
>    is unchanged (C2 set semantics), only the order of stderr entries changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: make "field produced but no one consumes" go from undetectable to a fact assertable
at compile time or test time. RFC-039 has already listed this as "the single most valuable item".

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
    /// Settlement: called once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three levels)**:

| Situation                                | Criterion                                                   | Disposition                                                           |
| ---------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------- |
| Obligation read by its declared consumer | The field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                  |
| Obligation non-empty but no consumer     | Field is non-empty and not consumed                         | **Produce diagnostic** (W-level by default; E-level in `strict` mode) |
| Obligation empty                         | Field is empty                                              | Pass (no need to consume)                                             |

**Why W-level first, not E-level**: fixing obligation discarding will **change the diagnostic set**.
C2 criterion requires "the diagnostic set is the same across entries"; if we jump straight to
E-level, `yaoxiang check` will suddenly produce a large batch of `Unproven` diagnostics that were
previously silent. Doing it in two steps (W observation first, then upgrade to E) makes the
criterion usable at every step. See "Implementation Points" S4 for details.

**The call site of `assert_drained()` is unique**: `Driver::run`, at the tail of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appears ≥2 times but has no third-file read → failure). `assert_drained()` is the **runtime** gate,
and that script is the **static** gate; the two are complementary, and both are needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in project, results filtered to target file)
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

**`stages()` can only be the 6 predefined combinations above (one per `ProgramKind`); it does not
accept the caller freely passing an array**. This is the key constraint that prevents the `Program`
abstraction from degrading into "any free parameter can be passed", and is asserted by
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
                state.record_skipped(stage);   // must produce diagnostic, see below
                continue;
            }
            match stage {                            // exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one fewer
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for record—
>
> 1. The `StageOutcome` three-state in §4 is not returned by the stage arm, but expressed by the
>    `State` failure mark + `Aggregation` gating (Continue / Warn / Abort semantics unchanged,
>    boilerplate returns per arm are saved);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the "avoid
>    Driver holding mutable global state" decision in §5 covers the sketch field);
> 3. `Skipped` diagnostics are only recorded into the `DriverOutcome.skipped` internal ledger in
>    4.1.3, not sent externally—S2's "deliberately fix no bugs" zero-diff criterion requires this;
>    external sending follows with the 4.3 obligations ledger;
> 4. The `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the sole
>    new caller (L1→L2 is an allowed direction); the orchestrator existing call sites' ownership
>    reversion will be discussed after the 4.2 migration is complete.
> 5. In the multi-file form, ProofExecution is an independent stage, executed after **all**
>    typechecks (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof within
>    the per-file typecheck loop (file N's proof before file N+1's typecheck). For single failure,
>    the two are byte-for-byte identical; for the "file 1 proof failure + file 2 typecheck failure"
>    multi-failure scenario, the first report changes from proof error to typecheck error (test
>    nailed board).
> 6. `DriverOutcome` carries products in separate channels by `ProgramKind`: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    each entry (`PipelineError` / `OrchestratorError`) are not types that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (each discovered file has an entry, clean files have an empty Vec—`check_project`'s current
>    contract); the Discovery product is extended to the "file set + used_by edge set + shadowing
>    events" triple entering State (the MultiFile form has no downstream consumer, only recorded,
>    not sent externally); on the check path, each file's parse drops from 2 times to 1 time
>    following the C3 topology.
> 8. Lsp form landing (4.2.3, ruling by file source split, 2026-10-09 user final): disk file parse
>    failure is treated the same as Check's Option B (collect + exit compilation unit—both ends
>    degrade to convergence); edited buffer parse failure retains the defective AST to continue
>    typecheck (editor philosophy—an in-the-middle-of-typing state should not make semantic features
>    disappear). The Discovery arm retains the buffer source overriding stale disk content for Lsp;
>    Typecheck under Lsp only checks the target file (other units are only there for the registry to
>    provide signatures); the in-file diagnostic order normalization principle extends to LSP
>    (typecheck diagnostics → W-code warnings → proof errors). The old behavior of LSP hard-stopping
>    on unrelated disk file parse errors, with the handler silently falling back to the single-file
>    path, is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` gains a new `shared_registry` field—the embedded std
>    module is a sub-compilation, the registry is an **input**, not a product (EMBEDDED_STAGES has
>    no Registry stage), so the SymbolTable sharing contract of #94 is thus externalized from "the
>    caller remembers to pass the same registry" to a program declaration. Error path text changes
>    from `<std.test> (embedded std)` to the unit virtual path `<std/test>` (only the compiler's
>    internal error surface, no test nailed it). Thus all four orchestrator entries migrate into the
>    Driver.
> 10. Standalone check unification (4.2.5, Ruling A + IR stage ruling, 2026-10-09): the Check form's
>     faithful carrying of single-file semantics for programs without a project root—the warning
>     surface only covers the entry file (neighboring files only collect errors, rather miss than
>     misfire); relative `use` resolves along the importer's directory (rustc single-file mod
>     alignment). The CHECK stage table adds GlobalSlotAlloc + IrGeneration: the standalone old path
>     (pipeline full chain) already runs IR generation; E3019/E1014/E1015 etc. are only produced in
>     ir_gen (runner gate empirical). The Check form of IrGeneration is pure check, does not consume
>     IR; the Script/Bin split is expressed by `module_key`'s None/Some (the existing switch for
>     E3023, ir_gen.rs:1660). Monomorphization does not enter CHECK (only produces resource
>     limit/internal errors, zero corpus dependency). **〔2026-10-10 withdrawn via 3.4.8 empirical
>     evidence〕**—"zero corpus dependency" is actually an evidence blind spot that the corpus has
>     no pathological recursive fixture; E3005 is the only compile-time line of defense for this
>     class of programs. See note #14. Incidentally fixes a latent defect: the Registry arm's
>     repeated harvesting of registration for the embedded std unit overwrites the native half
>     already merged by `with_std()` (E1043 false positive)—the embedded unit now skips repeated
>     registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is deleted, going to
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains the defective AST to
>     continue typecheck (editor philosophy extended from the 4.2.3 ruling; not marked as stage
>     failure, otherwise the topology skip would prevent typecheck from ever running); Typecheck
>     dispatches the `check_module_collect_all` free function. The LSP single-file path from here on
>     completes proof (E4018), W-code warnings (W1001–W1005, test nailed board), and IR-level
>     errors; lexical failure now reports real diagnostics (the old synthetic "E0001 lexical error"
>     text disappears with the sequence deletion—real diagnostics carry precise spans, with no
>     information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries had already converged independently—the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`; pass-3 and drain both modes run together, the difference is carried
>     by whether collected_errors is empty; the five collection points of statements.rs (function
>     body/use/for/block/while). During verification, the duplicate diagnostic defect was confirmed
>     and fixed: the collection point puts the first error in both collected_errors and the Err
>     return channel, pass-3's `add_error` on Err causes the same code, same span, same message ×2
>     (nested can be ×3); there are also two independent mechanisms: the annotation check signature
>     form parameter and the whole annotation double visit (E1003/E1103), the ownership layer
>     same-point double emit (E2014/E2018). The fix lands at the module result boundary: deduplicate
>     by (code, span, message)—diagnostics are positional facts, duplicates carry no information; 63
>     corpus entries have their duplicate copies removed, the run column and exit code have zero
>     drift (the "duplicate" of `borrow_conflict_err` is an artifact of the baseline triple format
>     not including column number—two E2018s are actually legitimate diagnostics at different
>     columns).
> 13. Wasm playground landing (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver via `compile_playground`—the `Compiler` wrapper no
>     longer passes through. Error text is aligned byte-by-byte with the old `CompileError` Display
>     prefix (Parse error:/Type error:/Internal error:), with zero visible difference in the
>     playground UI. WasmPlayground and SingleFile share `SINGLE_FILE_STAGES` + FailFast (4.1
>     existing declaration, this step gives it a real producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of the mono exclusion in note #10**). Empirical evidence overturns "mono
>     only produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) causes the
>     compiler process to stack overflow (0xc00000fd—the interpreter recurses at the Rust level, no
>     graceful runtime error) on the multi-file path without mono, and the mono depth gate is the
>     sole compile-time line of defense; standalone check used to go through the full pipeline chain
>     before 4.2.5 and had mono, the exclusion ruling caused a silent coverage regression ("zero
>     corpus dependency" only proves the corpus has no pathological fixture). The "type erasure
>     fallback" assumption is voided by RFC-033 reflection ruling—`^^List(Int)` needs the real
>     identity of instantiation, monomorphization is the foundation of reflection rather than an
>     optional optimization. Four design points: ① typecheck for qualified calls (`lib.f(x)`,
>     FieldAccess form) produces `instantiation_request`, `generic_id` uses the qualified name,
>     sharing namespace with merged IR (currently only the bare Var is recognized,
>     expressions.rs:2455); ② `Stage::ALL` topology changed to Linking before Monomorphization, the
>     MULTI_FILE stage table tail adds Monomorphization—consumes merged_ir, aggregates all-unit
>     requests (the deferred bucket's `containing_fn` qualified), mono upgraded from a single-module
>     pass to a whole-program pass; the single-file stage table is unchanged (no Linking,
>     subsequence property preserved, IR snapshot zero drift); ③ CHECK adds Linking (pure
>     merge—E3020 entry validation stays in RoleClassification, to prevent double reports) and
>     Monomorphization (pure check, no consumption, same as the Check form of IrGeneration) two
>     stages; ④ resource protection follows the whole-program BFS coverage of cross-unit mutual
>     recursion. The Monomorphizer body is unchanged (the contract of consuming ModuleIR + request
>     set is exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design, landing the qualified call
>     instantiation collection: `ExpressionInferrer` injects the module qualified key table (three
>     `use` arms registered—normal / alias / per-item alias, and only SubModule class exports),
>     `callee_generic_name` is stitched via `SymbolTable::qualify` to produce a qualified name with
>     the same source as merged IR; the qualified call's first path arity criterion is tightened
>     (when the parameter table cannot be resolved, it falls to the second path and takes actual
>     arguments by signature, to avoid cross-module same-name function mismatch and false-positive
>     E3018). During implementation, three asymmetric defects on the mono side were empirically
>     found and fixed in the same round: (1) The granularity mismatch between the deletion key (the
>     generic name set) and the rewrite key (the `containing_fn` + name + span triple)—call sites
>     that cannot resolve actual arguments degrade to symbol requests entering the deferred bucket,
>     and when the container function is non-generic, the bucket never drains, while the original
>     has already been deleted by name → dangling calls (`list_ops.yx:53` runtime E6006 empirical);
>     fixed by restoring the gate `restore_generics_with_uncovered_call_sites`, with
>     `build_call_site_map` as the single source of fact for coverage determination, and the
>     originals of uncovered call sites are preserved. (2) The recovery segment's `HashSet`
>     iteration order causes the function table order to be non-deterministic (t2 review F1) →
>     changed to `Vec` and sorted by name, the product is byte-level reproducible. (3) The
>     non-rewrite face forms TailCall / MakeClosure still look up the mapping key as usual (the
>     comment claims conservative) → `call_form_is_rewritten` single-point determination,
>     non-rewrite forms are always counted as uncovered. Each has a corresponding nailed test (red
>     state empirical discrimination before the fix). Corpus differential baseline regenerated with
>     the same commit to exactly one line (the `remove_at` frame adds `(int64)`). **Commit strategy
>     note**: when the typecheck collection and mono fix are split into two commits, the staging
>     tree containing only the former triggers a red for the existing defect in `stdlib_docs` on
>     HEAD (E6006 of the same family as `list.is_empty([])`)—the two halves are each other's
>     green-light prerequisite, so they are merged into a single atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **D2 ruling armor**—the user module
>     export carries `generic_fn_type_params_snapshot()` via `Export.type_params`, isomorphic to the
>     std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source of
>     qualified call monomorphization, and the signature reverse-deduction is downgraded to a
>     fallback (merged into the R2 scope). (2) **HKT declaration side alive, call side broken**
>     (inferred passing type argument E1002 / explicit type argument E1010) registered as WBS 3.5 (3
>     level-3 tasks). (3) **deferred bucket under non-generic container never drains** merged into
>     the acceptance sub-items of 3.4.8 ②: the bucket must drain or loudly error out, must not
>     silently not specialize. (4) **Closure variable by-name call runtime E6006** registered as WBS
>     3.6; semantics (i) support indirect call / (ii) compile-time rejection, **ruled as
>     (i)**—support indirect call: the `CallDyn` mechanism is already in place (`ir_gen.rs:7945`
>     closure expression path); the break is that the `Var` callee is downgraded to a by-name
>     `Call`, and typecheck already accepts this program. (5) **Trade-off criterion solidified**—the
>     reasons for rejecting a plan can only be based on correctness and readability, "large change
>     surface / introduce new mechanism / high cost" do not constitute reasons for rejection, fall
>     in `coding-rules.md` part two and HOWTO self-check list; trade-offs that need user decision
>     should be given all four items per `AGENTS.md` "When requesting user ruling (mandatory)"
>     (situation + evidence / per-item pros and cons / clear recommendation + cost / destructive
>     naming).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, b554497b + e60dcc87). ② Per #14 design landing:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appended mono;
>     the Linking arm **before taking each unit's IR** aggregates the whole-program requests and
>     qualifies `containing_fn` (otherwise the deferred bucket key and merged IR are not the same
>     source, the bucket never drains). **① closure (D2 ruling armor)**: the user module export
>     carries the declaration-stage type parameter snapshot via `Export::type_params`, isomorphic to
>     the std path (`yx_sources.rs:62/108`). **Defects empirically found and fixed during the
>     process**: (a) **F1**—`qualify_names` only rewrites the function body, `ir.init` is never
>     visited, multi-file function definitions are already qualified but module top-level calls are
>     still bare names → runtime E6006 (**reproduces even with non-generic**, unrelated to
>     generics); fixed by init reusing the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono side's "whole-program unique suffix" heuristic and
>     unmatched silent `continue` → delete the heuristic, unmatched changes to **loud E3018**
>     (ambiguous silent mismatch is worse than not parsing). **Independently verified regression
>     caught and fixed**: t6 found that after named import `use lib.{pick}`, the bare-name call is
>     mistakenly rejected by E3018 (the IR instruction name is the alias-qualified name, the request
>     name is the bare name, the aggregation side only qualifies by unit key) → fixed by the
>     aggregation side **directly calling ir_gen's** `import_aliases_from` and `resolve_call_target`
>     (extracted as a single `pub` point, one producer one consumer), and matrix nailed tests cover
>     both forms of named import. **Filing**: the empty `DefId` of the init segment is empirically
>     shown to be a "consistency blank" rather than a live defect (the codegen name fallback can
>     resolve, all four init calls measured pass), registered as WBS 3.7.4 to close the "function
>     body goes through def, init relies on names" caliber fork.
> 18. Init segment DefId caliber fork closure (3.7.4, 2026-10-11, b7ac7396): `assign_defs` extracts
>     `resolve_static_targets` as a single point (Call/TailCall string operand + MakeClosure), the
>     function body and `ir.init` share the same implementation and the same `resolve` closure;
>     after 208 IR snapshots are regenerated, only 309 `(def=_)`→`(def=dN)` changes (other-lines 0;
>     whole-tree init segment 0 residual `(def=_)`). The codegen's DefId main path and name fallback
>     two states produce the same translation result and execution value (all four init calls
>     measured), the corpus baseline has zero drift; the probe is flipped and strengthened per the
>     new caliber (identity + exact index + execution value, fallback init coverage would turn red).

**Migration scheme for the ten entries (function-by-function specification)**:

| Entry                                                      | After Migration                                                                                                                                                                                                          |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body, change to construct `Program { kind: SingleFile, … }` and hand it to the Driver                                                                                                                |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                  |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                            |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                              |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter result to target file                                                                                                                                                                 |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                                |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                                   |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                                          |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In project: change to `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence of the single-file branch is deleted**, change to call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                            |

**`check_project` and `compile_project` have about 40% sharing / 60% forking**. The shared part:
vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`, parsing
loop, per-file checker assembly, result handling. The forked part: items 2/3/4/5/7/8/9/10/11 of the
11 items in the table above. **This 40% shared skeleton is exactly the value zone of `Driver`**—the
forked 60% is all "different stage sets" or "different aggregation modes", and these are exactly
what the two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "the upstream failed, so this stage was skipped" is not a stage return
value.**

The reason: skipping is determined by **topology**, not by the stage itself. If each stage were to
return `Skipped` itself, then the information "why didn't I run" would be scattered across the 12
stages and could not be centrally audited. Instead, `dispatch` determines it at the start of the
loop by dependency:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the vulnerability that this
document is meant to fix—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a
silent "skip" that produces no diagnostic, simply because the obligation happens to be empty. Once
the rule is established, any "didn't run Y because X wasn't done" must be explainable.

The diagnostic text needs to distinguish three reasons for skipping:

| Reason                                                    | Text Direction                                                                                                                              |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                                | "Stage Y was not executed because the upstream stage X failed"                                                                              |
| Caused by empty upstream obligation                       | "Stage Y was not executed because there were no pending obligations (normal)"—**this item's severity = Info, not counted in warning count** |
| Conditions not met (e.g., `config.mono.enabled == false`) | "Stage Y was not executed because the configuration was not enabled"                                                                        |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of that count).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop at the first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                             | Current State Correspondence |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error return `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early returns)               | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
project-internal files and single files within the same process, and the CLI's `run` and `check` are
two independent calls; putting them on Program avoids the Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a "whether to early
return" difference—they are two different checker entries. After unification, the same
implementation should be driven by the `Aggregation` parameter rather than retaining two functions
(**this is an additional item that this document needs to verify, see "Known Limitations and
Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation introduced, no touching of `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what migrates into the ledger is **settlement
responsibility**, not the module to which the type belongs. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the rest of the field types stay
where they are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`.
L3 consumers (such as `ir_gen` reading `release_plan`) continue to receive the specific field type
via parameters, **must not `use crate::driver`**—this is consistent with the
driver-no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse
`use crate::driver`**. The `Driver` appearing in `TypeChecker`'s import is treated as a violation,
intercepted by `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Migration                         | After Migration                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                                                                                     |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution is already plugged in by P3's stop-the-bleed; **the warning surface remains not newly added** (Decision B1, 2026-10-09: dead code family warnings do not enter the compile path—the pool semantics is being corrected by 4.9's identified defect, the normalized warning surface waits until after 4.9; the original draft's "newly added warning output" for this row is voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already plugged in by P3's stop-the-bleed                                                                                                                                                                                                                                                                                                                                                         |
| LSP (in project)                    | No proof_execution                       | Already plugged in by P3's stop-the-bleed                                                                                                                                                                                                                                                                                                                                                         |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                                                                         |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                                                                                            |

**The only intentional behavior break** is changing the `.expect()` at `predicate.rs:35` to return a
diagnostic. This conforms to RFC-027 §8's "not bound to a specific solver" positioning, and is also
the contract already declared by `backend.rs:60` ("caller should degrade conservatively")—the
current implementation is the opposite of its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (single-file: 1 / multi-file: N)                               |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                            |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                   |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **preserved**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                               |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to go through Driver                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or changed to be driven by `Aggregation` parameter                                                                                                                                                                                                                                             |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                                              |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                                  |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                            |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix the second silent discard site**)                                                                                                                                                                                                                                                            |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | The `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct the comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "lower layer failure means upper layers do not run" (no short-circuit exists)                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumer sites of `predicate.rs`/`checker.rs`/`ownership.rs` changed to go through the shared port. The `&'static` bare reference form is not feasible (`dyn Solver` is not `Sync`), the original text expression follows D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448` (the logic
of these six branches is itself correct—**the problem is on the consumer side, not the producer
side**, modifying the producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (reachability determination of the wasm branch belongs to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                      | Compatibility                                                              | Disposition                                                                                                                                     |
| ----------------------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                        | **Breaking**: previously silently-passed constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; batch release, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                      | **Breaking**: previously-compiling programs start producing warnings       | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), does not change exit code                       |
| `assert_drained()`'s first run produces W-level diagnostics | **Breaking**: diagnostic set is newly added                                | W first, E later, S2/S4 split into two steps                                                                                                    |
| `predicate.rs` panic changed to diagnostic                  | **Improvement**: no longer crashes                                         | No break                                                                                                                                        |
| `build` / `dump_bytecode` subcommands                       | **No impact**                                                              | These two paths do not go through proof_execution                                                                                               |
| `TypeCheckResult` fields migrate to `Obligations`           | **Internal refactor**                                                      | If the `pub` API surface has external dependencies, they need to be synchronized; in-repository consumers are all listed in the change list     |

---

## Implementation Points

This document corresponds to **P3 (Fix Correctness Vulnerabilities)** and **P4 (Stage Contracts and
Unified Driver)** in RFC-039's global stage sequence. The following S1–S5 are the implementation
order within this document, **each step's acceptance criterion cites the C2 category of the
[equivalence criteria document](07-equivalence-oracle.md)** (orchestration changes: **diagnostic
set** the same across entries + corpus behavior the same).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** The vulnerability criterion must be written as failing first.

| Deliverable                                                                                                                                                 | Acceptance                                                                                                            |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                               | **Must be red**. If it is unexpectedly green, it means this document's vulnerability analysis needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Ruling D48—do not mix into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of the single-file and multi-file two versions of the corpus are comparable                           |
| `test_obligations_drained` skeleton                                                                                                                         | Mark `#[ignore]`, turn green in S4                                                                                    |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path's diagnostic set and exit code are byte-by-byte identical**                  |
| Whole corpus (293 `.yx`) differential                    | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately fixes no bugs**—it only moves the existing behavior into the Driver. The
acceptance standard is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` is restored to its original state.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The four entries' diagnostic sets are the same after normalization by `Aggregation` |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project                |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                          |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                                 |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce diagnostic   |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in Z3-missing environment                                          |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If the
structural merge has issues, you can roll back only the structural commit while keeping the
vulnerability fix commit—at this point `test_multifile_proof_obligation_not_dropped` stays green;
the reverse (keep structure, roll back fix) will turn red, which is an unacceptable intermediate
state, and is forbidden to be merged.

### S4: Enable Obligations Ledger

| Deliverable                                  | Acceptance                                                          |
| -------------------------------------------- | ------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                              |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                            |
| Obligation diagnostic W → E upgrade          | After whole corpus differential, manually review each newly added E |

**Rollback point**: the severity of `assert_drained()` is controllable by a configuration item; the
W/E switch does not require code structure changes.

### S5: Proof Layer and Wasm Wrap-up

| Deliverable                                                                                                                                                                                                                                                    | Acceptance                                                                                                                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                                         | Documentation corresponds to `checker.rs` call sites one by one                                                                                                                                         |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise a single timeout would be固化 across compilations through the process-level cache and pollute the shared state between cargo test threads) | The `default_solver()` call sites on the production path go to zero (only tests keep it); the cache hit rate is observable across the three consumer sites (the counter is already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                                                | Has diagnostic when there is no `body_checker`                                                                                                                                                          |
| Reachability determination of the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                                      | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination need                                                                                                       |

**Note**: correcting the layer order will change the diagnostic set and may expose a large batch of
previously-silent `Unproven`. **This item should be walked independently of S1–S4**, do not mix it
into the same PR as the entry merge.

## Key Decisions and Rationale

| Decision                 | Decision                                                                                                                                                                 | Rationale                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **refuse runtime registration**                                                                                    | Exhaustive `match` is a zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism that the opcode table has been verified in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating the call-count question that requires human reasoning           |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, used in parallel with the **static** gate of `scripts/ci/check-obligations.py`; severity W first, E later | What is fixed is an entire class of bug, not one bug: the same class of hidden danger has at least 2 confirmed locations (`proof_calls`, `checker.rs:1313`), all 16 span-keyed fields are in range. W first, E later, is to make the C2 criterion usable at every step—jumping to E would make `yaoxiang check` suddenly produce a large batch of previously-silent diagnostics |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **don't touch** the producer-side logic of the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448`    | The problem is on the consumer side, not the producer side. Modifying the producer side would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                         |

This design additionally eliminates two types of cracks:

- **The crack between "comment promises" and "code behavior"**. "Must not let silent pass come back
  to life" at `checker.rs:5165` and "lower layer failure means upper layers do not run" at
  `layers/README.md:3` are both **comment-level contracts**—the former is redeemed by
  `assert_drained()`, and the latter is exposed by topology-driven `Skipped` diagnostics.
- **"Forget to append to one array"**. Currently five functions each hand-write the call sequence;
  after unification, it is a single stage table, and a newly added stage being missed will cause a
  compile error.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type interfacing**, they are configuration questions of
  "whether to run a certain analysis"; it also cannot express `Aggregation`, it can only move the
  either-or of `check_module` / `check_module_collect_all` from the function layer to the type
  layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, equivalent to
  swapping "five functions each hand-write the call" for "one array forgot to append", which is
  exactly the cause of the current 11 inconsistencies. This repository has the same-form
  anti-example: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs without changing structure**—can fix the known 2 bugs, but only covers 1 of the 16
  obligation fields; `Stage` is still scattered across 5 functions, the next fork point will
  continue to grow from here. **It must be done first** (part of S1/S3), because the structural
  transformation needs a known-red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a general entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is looking for the bug at the wrong
  level: the bug is in the orchestration layer, and the detection must be in the orchestration
  layer.

## Known Limitations and Risks

- **The slim-down of `orchestrator.rs` will make it harder to read in the short term**. After 139
  lines of `compile_project` is split into "Program constructor + several driver arms", the reader
  needs to cross two files to understand the flow. This is the common cost of all "centralize the
  wiring" refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent issues"**. After the fix, there may be a batch of "new errors" user reports—they
  are real bugs that just weren't reported before. This must be clearly stated in CHANGELOG.
- **The severity switch of `assert_drained()` requires field-by-field manual judgment**. In the 16
  fields, some (e.g., `module_namespaces`) having "no consumer" may be a design where no consumption
  is needed, and should not be alarmed. S4 needs to go through them one by one, not a
  one-size-fits-all approach.
- **The `Program` abstraction may be premature**. If the stage set of some entries is unstable for a
  long time, `stages()` will degrade to "every call passes a different array" free parameter, and
  the contract constraint will fail. **The mitigation is `test_program_stage_coverage` asserting
  that `stages()` can only come from the 6 predefined combinations.**
- **Criterion dependence**: S1/S3's vulnerability fix criteria depend on the vulnerability tests of
  the [equivalence criteria document](07-equivalence-oracle.md) being red first. **If that document
  does not first establish the multi-file corpus layer, S1 cannot be accepted**—because the existing
  293 corpora all take the single-file path and have zero coverage of this class of defect. This
  document does not involve the IR verifier (`verify_loose`), and that part of the prerequisite work
  is not in the scope of this document.
- **Correcting the layer order will expose previously-silent `Unproven`** (`equivalence` is not in
  the pipeline at all, `termination` and `ownership` are in reversed order relative to the
  declaration). This is a diagnostic set change risk and should be walked independently.
- **Thread safety after the SMT backend is changed to a singleton is undetermined**.
  `backend.rs:13-19` already explains that `Solver` is only `Send` and not `Sync`, and `Z3Backend`'s
  cache is `RefCell`; after changing to a singleton shared across compilation units, it needs to be
  confirmed that all access paths go through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). It should be opened only after the equivalence criteria are
  stable.

> **The open questions originally listed in this section have all been ruled on.** The
> decision-by-decision list is in the
> [RFC-039 ruling registration](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` transformation (construction steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design origin of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Comparison code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "must not let
  silent pass come back to life"
- `src/frontend/core/typecheck/layers/README.md:3` — The explicit declaration of "lower layer
  failure means upper layers do not run" (no short-circuit actually)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — The explicit reason for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test self-admits
  "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus
  self-admits "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path,
  therefore executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Error code build-time gate, an example of this project's enforcement mechanism
