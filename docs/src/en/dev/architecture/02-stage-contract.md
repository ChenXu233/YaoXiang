# Compilation Stage Contracts and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion tiers, and execution stage sequence are defined in the
> RFC-039 main text; the positioning of each subsidiary document is described in
> [this directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses a
single problem: **"Which stages did this compilation run, which did it skip, and why" is currently
spread across six paths and ten entry functions, each wired by hand, with no single place that can
answer the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are violated by the architecture rather than the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally reads "Unproven → compile
error, no degradation, no silent pass……do not let silent pass return." But this invariant only holds
on the **single-file path**: a proof function constraint like `x: Sorted(3)` **silently passes** on
the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths — the proof function is never
executed, and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, no runtime registration) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-function refactor plan for all ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and the diagnostic aggregation pattern
  (`FailFast` / `CollectAll`);
- All files and line ranges touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency direction rules, and anti-regression gates → `01-routing.md`
- Equivalence criteria (C1-C6 tiers, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- Global P1-P10 execution order and G1-G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 provides the **why** of the refactor and the **order** in which it should be done; this
document provides the **concrete form of L1**, the **per-file change list**, and the
**implementation stages within this document** (corresponding to RFC-039's global sequence P3 "fix
correctness bugs" and P4 "stage contracts and unified Driver"). Where this document conflicts with
RFC-039, RFC-039 takes precedence.

Equivalence criteria are executed under the **C2 (orchestration changes)** category of the
[Equivalence Oracle document](07-equivalence-oracle.md): same diagnostic set across entries + same
corpus behavior.

## Current State

> **All facts in this section are verified**, each with file path + line number. Line numbers are
> based on `9e02e4db`.

### Stage Boundaries Are the Only "One Wrong Place, Whole Repo Silent" Structural Defect

It differs in nature from the other two defect classes (module boundaries, test wiring):

| Defect                  | Typical Manifestation       | Any Signal?              |
| ----------------------- | --------------------------- | ------------------------ |
| Lexical/syntactic error | Source code written wrong   | Has diagnostic           |
| Type mismatch           | Type written wrong          | Has diagnostic           |
| Stage skipped           | Some stage was never called | **No signal whatsoever** |
| Obligation not consumed | Field filled but never read | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. They therefore cannot be
solved by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what was not written**. This is exactly the root cause identified by RFC-039:
"this project treats 'design' as a documentation convention rather than an executable constraint."

### Existing Enforcement Mechanisms in This Repo

The same repo already has mature enforcement mechanisms — they just haven't been extended to the
stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subjected to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, comparing each one against
  the RFC-013 code table and `panic!`-rejecting the build on any mismatch.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` is 6227 lines / 47.9%) has high test
  density; every module's test subtree is wired up by declaration — this is the most completely
  wired block in the repo and can serve as the formal reference for stage-layer gates.

**The design capacity is there. What's missing is "placing an equivalent-grade gate in the
orchestration layer as well."**

(Line count basis: `(Get-Content).Count`, see the "Line count basis" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                 |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | Direct 5-stage                                    |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                  |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking            |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics       |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + standalone IR           |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1                 |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                     |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes to 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                     |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1          |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                       |
| ------------------- | ------------- | -------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                        |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below provides evidence cell by cell. **A blank cell does not mean "this entry does not do
this", but "no code in this entry does this"** — this is the very essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                      | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`            |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------ |
| 1   | **proof_execution**                                      | **yes** `187-203`                                | **no**                                                 | **no**                                                      | **no**                             | **no**                               |
| 2   | Dead code analysis                                       | yes `275-278` (`config.dead_code.enabled` gated) | **no**                                                 | yes `356-377` (role-aware, no config gate)                  | **no**                             | **no**                               |
| 3   | W1006 local module shadowing                             | **no**                                           | **no**                                                 | yes `336-348`                                               | **no**                             | **no**                               |
| 4   | W1003 unused import                                      | yes (`277` collects `type_result.warnings`)      | **collected but never output**                         | yes `350`                                                   | **no**                             | **no**                               |
| 5   | W1001/W1002 dead-code family                             | yes (same as 2)                                  | **no**                                                 | yes (same as 2)                                             | **no**                             | **no**                               |
| 6   | **monomorphization**                                     | yes `381-389` (`config.mono.enabled` gated)      | **no**                                                 | not applicable                                              | not applicable                     | **no**                               |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first-error return** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)              |
| 8   | File discovery                                           | not applicable                                   | `discover` (`101`, drops `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | not applicable                       |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **no**                                           | **no**                                                 | yes (`321-328`)                                             | **no**                             | **no**                               |
| 10  | Global slot allocation                                   | not applicable                                   | yes `allocate_global_slots` (`147`)                    | **no**                                                      | **no**                             | **no**                               |
| 11  | IR generation + qualified name rewriting + linking       | yes (`205`)                                      | yes (`152-236`)                                        | **no**                                                      | **no**                             | yes (standalone ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline, preserved as-is. Differences in current state after P3
> landing:
>
> - **Row 1 has been fixed**: proof_execution shares the same implementation in
>   `frontend/proof_execution.rs` across five entries (pipeline + four orchestrator entries); the
>   single-consumer defect no longer exists.
> - **Rows 2/4/5 remain unchanged** (multi-file `run` still doesn't output W1003, doesn't run the
>   dead-code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization exclusive to single-file) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3, potential unverified risk).
> - **Row 7** check_module / check_module_collect_all dual entry → assigned to 4.2.7 (Aggregation
>   parameter driven).
> - The "Ruling #434" referenced in code was previously unregistered in docs — now registered as
>   RFC-039 **D57**.
> - **Embedded std pulled into the asymmetry (new out-of-band fact, registered 2026-10-09)**: the
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, hard switch by #117); the multi-file `discover` previously only recognized
>   explicit `use` — for-loops in projects compiled successfully but produced runtime E6006
>   (probe-verified). Fixed and WBS 4.10.2 retired; the same-family "multi-file missing single-file
>   step" parse-hard-abort oddity filed under WBS 4.10.1 — **also fixed** (landed right after 4.2.2;
>   the Check path degraded to per-file collection; from the audit method's "stage coverage / field
>   consumption" view, this is a compilation-unit membership difference — a slipped-through
>   dimension).
>
> > **Review note (2026-10-07)**: the hard-failure panic has been eliminated by P3's 3.3.1 (SOLVER
> > slot made Option, missing → conservative degradation to `SMTResult::Unknown`); "silently skip
> > (no injection)" is supplemented with W1081 signal by 3.4.2. Unification of the three forms
> > (singletons) is executed per **RFC-039 D58**, with the actual fix location at
> > `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure
> > access); the change-list expression in 02, `default_solver() → &'static`, is governed by D58
> > (bare `&'static` reference is not viable: `dyn Solver` is not `Sync`).

**Two places need precise wording, otherwise the description will be wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report them. The
  reason is at `compile_project:138` — `type_results.push(result)` stores the complete
  `TypeCheckResult` (including `warnings`), but this function **has no `result.warnings` read site
  anywhere in its body**; the only downstream use of `result` is `generate_ir_with_context`
  (`154-156`).
- **The `main` criterion for the entries in row 11 also has different sources**:
  `check_project:385-395` uses `surfaces.bins` (manifest-declared surface); `compile_project` uses
  `is_bin_role` (`250-252`, only checks "is there a manifest"). The two functions give different
  answers to "which files must define `main`".

### Correctness Bug: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are reproducible.**

**Step 1 — The obligation's production point is unique.**
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

The semantics is: "the predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed to determine the result."

**Step 2 — The consumption point is inside the checker, but only produces, doesn't consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                   |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **produces no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **produces no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **produces no diagnostic** |

The comment at `checker.rs:5165-5169` literally reads:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch thus becomes a true defense line rather than a forward-looking stopgap: do not let silent pass return.`

That is: the author knew full well that "producing no diagnostic" is a defect, and explicitly
classified it as a **known intermediate state with nowhere to go**, with the design assumption that
"someone will come to read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects into it; `checker.rs:1439` writes to `TypeCheckResult` with
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repo.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is read in production code at **only one place: `src/frontend/pipeline.rs:187`**
(passed as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repo fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` are producer-side collection;
`verdict.rs:61` is a same-named field on `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries skip that layer.** `src/frontend/module/orchestrator.rs`'s
`compile_project` (`99`), `check_project` (`273`), `check_source_in_project` (`450`), and
`compile_embedded_module` (`1374`) **all do not call `pipeline.rs`**, calling
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they cannot even reach the
sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: the embedded standard library's own refinement obligations also take the
discarded path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible
for compiling the embedded std. This means **the proof obligations of the embedded std module itself
are likewise never executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles and runs successfully with no diagnostic
whatsoever** on the multi-file / check / LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layer.**
When `checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` from the ownership check layer is swallowed by an empty match arm — no diagnostic,
no bookkeeping.** This directly contradicts the "do not let silent pass return" promise at the
`5164` branch, and is **independent of the orchestrator issue** — even if the entry layer were
completely fixed, this one would still be silent. (Handling timing: should be processed in the same
batch as the obligations ledger, because it's the same class of problem.)

### Why the Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three single-file tests in RFC-027
(`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`, `rfc027_refined_transparency.rs`,
`rfc027_return_refinement.rs`) do assert that `proof_calls` is non-empty — for example
`rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call exists in `result.proof_calls`.
But they **all go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping exactly
before `pipeline.rs`**.

The test file's own doc-comment already states this. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof invocation (E4018) is executed by pipeline.rs after check_module, so the refinement-violation cases are in the .yx layer`

**This is exactly the shape of the problem: the test verifies "obligation was filled in", while the
bug is "consumer didn't read it".** A test that only tests the producer, not the consumer, is
naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Oracle's Main Corpus Is the Single-File Path

The [Equivalence Oracle document](07-equivalence-oracle.md) treats the end-to-end differential of
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion** and the main acceptance
means for C2 stage changes. But actual measurement:

- The `tests/` directory has **no `yaoxiang.toml`** (zero hits repo-wide via glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is empirical proof of
this. The file is annotated `// expect: compile-error E4018` (line 15), and the comment at line 7 in
its header reads "Status: ❌ Should be rejected at compile time" — **it passes precisely because it
takes the only path that executes the proof function**.

**Conclusion: the third-layer criterion needs a supplementary multi-file corpus layer, otherwise it
cannot serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 16 "Produce-as-Contract" Fields, Zero Mechanism Guarantees

In `src/frontend/core/typecheck/types.rs:16-70`'s `TypeCheckResult`, there are **16 fields** whose
doc-comments explicitly say "produced by stage X → consumed by stage Y" — they are in essence
**cross-stage obligations**:

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

**All these fields are passed across layers keyed by `Span` or name tables.** The Equivalence Oracle
document already points out that mismatches in the span-keyed contract "silently fail, with no error
whatsoever" (`ReleasePlan` is typical: any inconsistency in how the two sides compute span → `Drop`
instructions silently disappear).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
coincidental**:

- `release_plan`'s consumer at `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`) is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the consumer
  is in a downstream module, and that downstream module is on the necessary path for every entry.
- `proof_calls`'s consumer is at the **top of `pipeline.rs`**, which is in **another entry
  implementation**. The four orchestrator entries do not even pass through `pipeline.rs`.

**Verified fact**: currently the entire repo has **no mechanism** to guarantee that any of these 16
fields is consumed. The only "protection" is that `ReleasePlan` happened to piggyback on IR
generation.

### Four Additional Contract Defects in the Proof Layer

The following all come from proof-layer analysis, are independent of the entry-fanout problem, but
all belong to the "stage contract is not enforced" class.

**(a) Declared layer order contradicts actual execution order, and the `equivalence` layer is not in
the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Depends On    |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

Line 3 of the README says "executed in layer order; if a lower layer fails, the upper layer is not
run." Actual call sites in `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Declared Layer |
| ------------ | ------------------------------------------------- | -------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2        |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1        |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3        |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0        |

That is: the order of `termination` and `ownership` is **opposite** to the declaration; the declared
Layer 0 `equivalence` **does not appear at all in the `check_module` stage sequence** (it is used
only as an `is_subtype` utility function in `inference/assignment.rs:17`, unrelated to
`ProofResult`). Meanwhile **there is no short-circuit whatsoever** — `checker.rs:1271-1276`
`add_error`s the termination errors one by one and then continues, so the README's promise of "if a
lower layer fails, the upper layer is not run" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: the hard-failure panic has been eliminated by P3's 3.3.1 (SOLVER
> slot made Option, missing → conservative degradation to `SMTResult::Unknown`); "silently skip (no
> injection)" is supplemented with W1081 signal by 3.4.2. Unification of the three forms
> (singletons) is executed per **RFC-039 D58**, with the actual fix location at
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure access);
> the change-list expression in 02, `default_solver() → &'static`, is governed by D58 (bare
> `&'static` reference is not viable: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Is Unavailable                                 |
| ------------------------------------------------------ | -------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Construction-time injection via `with_solver_owned`      | `None` → **silently skip** (no injection)                  |
| `ownership.rs:627` (back-edge cut determination)       | **Every call** `default_solver()`                        | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "initialization failure maintains
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond kernel
capability'." But the contract doc at `backend.rs:60` says "None: backend unavailable……**caller
should degrade conservatively**". **Two philosophies coexist in the same code, and the hard-failure
one panics instead of returning an error.**

**(c) A new Z3 context is created on every back-edge determination; the cache is non-functional.**
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

**This is a factory function, not a singleton** — every call `Z3Backend::new()`, i.e., creates a new
Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc-comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits in this call mode**.

(For contrast: the `LazyLock` at `predicate.rs:34-36` is a real singleton. Same backend, two
different lifecycle strategies.)

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

When `body_checker` is `None`, the ownership check receives an **empty type ledger and empty call
table** and runs as usual — ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least
left a trace in the obligations ledger.)

The comment at `checker.rs:1244` claims termination "runs after typecheck and before constraint
solving". Actually `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at `1256`,
ownership at `1296`, which is two layers after the "before" claimed in the comment**.

### wasm Status: Carried by shim crate, 27 files of branches are live

The wasm target **has been built and is built in CI**, and the `cdylib` is in a separate shim crate
rather than the main crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                        |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                              |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                              |
| Shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                     |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (the non-wasm counterpart at `Cargo.toml:125-131`: tokio/ureq/tempfile)                                                                                                           |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                        |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library pre-built by Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to warning on missing)                                                                                                                                                            |
| **27 files** contain the `wasm32` literal         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are doc-only mentions                                                                                                                                          |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                      |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                      |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 27 files carry weight.** They
determine which APIs in the main crate can be called by `wasm/src/lib.rs` (73 lines) under the wasm
target — `lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole (all
three need `std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Path consuming `proof_calls`               | Executes proof functions? |
| ------------------------------------------ | ------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **yes**                   |
| wasm playground (`wasm/src/lib.rs:48`)     | **yes**                   |
| `build_bytecode` (`lib.rs:171`)            | **yes**                   |
| Multi-file `run` (`compile_project`)       | **no**                    |
| `check` (`check_project`)                  | **no**                    |
| LSP in-project (`check_source_in_project`) | **no**                    |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "it has
three"**, but all three are on the single-file side. The `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see "Target Design" §3), otherwise the unified Driver will miss this path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — is assigned to `06-cleanup-inventory.md`. The RFC-039 open question "build
wasm target, or delete these branches" **is obsolete**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (exhaustive, no runtime registration)

**Core constraint: `Stage` is a compile-time exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can compile-time force "new stage must be handled by
the orchestrator layer" — this is exactly the mechanism the opcode table has already proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                   Project
    Parsing,            // Lexing + syntax                  PerModule
    Registry,           // Module registry construction     Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type checking (with embedded proof layers) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis        Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation           Project
    IrGeneration,       // AST → ModuleIR                   PerModule
    Monomorphization,   // Monomorphization                 Project
    Linking,            // Cross-module linking / IR merge  Project
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
at project level) make "which stages must be run per module, which must be run once at project
level" a type-level fact. The `Project`-scoped stages are structurally guaranteed to run only once
in `dispatch` — this eliminates questions like "should `allocate_global_slots` be called once per
file" currently in `orchestrator.rs` that require human reasoning.

**The stage table is in topological order, not alphabetical**, because failure propagation depends
on order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the initial draft corrected
> by implementation evidence —
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the initial draft order contradicts
>    the data flow.
> 2. The Check form's stage table is normalized to **dead-code before proof** by `Stage::ALL`
>    topological order: `check_project` currently runs proof before dead code; the two have no data
>    dependency and the diagnostic set is the same (C2 set semantics); only the per-file diagnostic
>    order is normalized. The single-file path (dead-code inlined into typecheck, before proof)
>    already matches the ALL order, and per-byte acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    initial draft order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; it also drops multi-file parse from 2 passes to 1. Status quo honestly
>    recorded as a side note: the multi-file path's parse error used to be a hard abort (the `?`
>    propagation in `build_registry_from` and the tension with CollectAll semantics are filed as WBS
>    4.10.1) — **4.10.1 has been fixed** (2026-10-09 user ruling on rust-style collection
>    semantics + plan B): the Check path's parse failure degrades to per-file diagnostic collection,
>    with sick files exiting the compilation unit (not entering the registry, importers report
>    E5001); the MultiFile path retains hard abort (under FailFast a bad file can't produce IR,
>    which is correct semantics, long-term nail-board effective).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. data-dependency
>    edges 14→16: `RoleClassification` consumes Discovery's used_by edge set, `DeadCodeAnalysis`
>    consumes Discovery's W1006 shadow events — the initial draft missed these (these two products
>    were produced by `discover_with_used` in the orchestrator era and dropped by the `discover`
>    wrapper, and the data flow got lost by not entering the table); b. The per-file diagnostic
>    order normalization is extended to the full form of this note's #2: E3020 entry validation
>    moved into the RoleClassification arm (check has no Linking stage, so the arm that holds
>    surfaces and AST does the validation), and the per-file diagnostic order = stage topological
>    order (E3020 → typecheck → W1006/ W1003/dead code → proof); the diagnostic set is unchanged (C2
>    set semantics), only the stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but never consumed" from undetectable into a compile- or
test-time assertable fact. RFC-039 has listed this as "the single most valuable item".

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
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Case                                 | Criterion                                                   | Disposition                                                                     |
| ------------------------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Obligation read by declared consumer | The field in `Obligations` is `take()`-en / marked consumed | Pass                                                                            |
| Obligation non-empty but no consumer | Field non-empty and not consumed                            | **Produce diagnostic** (`W` level by default; promoted to `E` in `strict` mode) |
| Obligation empty                     | Field is empty                                              | Pass (no consumption needed)                                                    |

**Why W level first, not E level**: fixing obligation discarding **changes the diagnostic set**. C2
requires "same diagnostic set across entries"; if we promote to E in one step, `yaoxiang check` will
suddenly produce a flood of previously-silent `Unproven` diagnostics. Two steps (W for observation,
then promote to E) keeps every step's criterion usable. See "Implementation Points" S4.

**The single call site of `assert_drained()`**: `Driver::run`, at the tail of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 already proposed `scripts/ci/check-obligations.py` (field appears
≥2 times but no third-file read → fail). `assert_drained()` is the **runtime** gate; that script is
the **static** gate; the two are complementary and both are required.

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

**`stages()` can only be one of the 6 predefined combinations (one per `ProgramKind`); the caller is
not allowed to pass an arbitrary array.** This is the key constraint that prevents the `Program`
abstraction from degenerating into "free parameters that accept anything", asserted by
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
                // … 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The only settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: four implementation deviations
> from the sketch above, semantically equivalent, registered for the record —
>
> 1. The `StageOutcome` three-state from §4 is not returned by stage arms but expressed by `State`
>    failure flag + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saving
>    per-arm boilerplate returns);
> 2. `Driver` has no `config` field — the only source of config is `Program.config` (§5 "Avoid
>    Driver holding mutable global state" ruling overrides the sketch's field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded in the internal `DriverOutcome.skipped`
>    ledger, not emitted — S2's "deliberately fix no bug" zero-diff criterion requires this;
>    emission follows with the 4.3 obligations ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is an allowed direction); orchestrator's existing call sites will be discussed
>    after 4.2 migration.
> 5. Under the multi-file form, ProofExecution is a separate stage, executed **after all** typecheck
>    (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof within the
>    per-file typecheck loop (file N's proof before file N+1's typecheck). Single failure is
>    byte-identical; in the "file1 proof fails + file2 typecheck fails" multi-failure case, the
>    first report changes from a proof error to a typecheck error (test nail-board).
> 6. `DriverOutcome` carries products in ProgramKind-specific channels: `result` (pipeline contract)
>    / `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type-face the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry; clean files have an empty Vec — the `check_project` status
>    quo contract); the Discovery product is extended to a triple "file set + used_by edge set +
>    shadow events" entered into State (no downstream consumer in MultiFile form, recorded but not
>    emitted); per-file parse in the check path drops from 2 to 1 pass with the C3 topology.
> 8. Lsp form landing (4.2.3, file-source triage ruling, 2026-10-09 user final word): disk-file
>    parse failure follows Check's plan B (collect + exit compilation unit — both ends converge in
>    degradation); edited-buffer parse failure retains a partial AST and continues typecheck (editor
>    philosophy — typing mid-state shouldn't make semantic features disappear). The Discovery arm
>    for Lsp reserves the buffer source code to override stale disk content; Typecheck under Lsp
>    only checks the target file (other units only supply signatures via the registry); the per-file
>    diagnostic order normalization principle extends to LSP (typecheck diagnostic → W-code warning
>    → proof error). The old LSP behavior of hard-aborting on parse errors of unrelated disk files
>    and the handler silently falling back to the single-file path oddity is fixed by this step.
> 9. Embedded form landing (4.2.4): `Program` adds a `shared_registry` field — the embedded std
>    module is a sub-compile, and the registry is **input** rather than product (EMBEDDED_STAGES has
>    no Registry stage); #94's SymbolTable sharing contract is thereby made explicit from "caller
>    remembers to pass the same registry" to a program declaration. The error-path text is
>    normalized from `<std.test> (embedded std)` to unit virtual path `<std/test>` (only internal
>    compiler error face, no test pinning). At this point all four orchestrator entries are migrated
>    into the Driver.

**Refactor plan for the ten entries (per function)**:

| Entry                                                      | After Refactor                                                                                                                                                                                               |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                        |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                      |
| `compile_project` (`orchestrator.rs:99-237`)               | Thinned to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                |
| `check_project` (`orchestrator.rs:273-399`)                | Thinned to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                  |
| `check_source_in_project` (`orchestrator.rs:450`)          | Changed to one Driver call + result filtered to target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Changed to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                   |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (forwards to `run_project` or `SingleFile`)                                                                                                                                                        |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (forwards to Driver)                                                                                                                                                                               |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project goes to `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch, change to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete `standalone` branch (`614-616` → `check_single_file` `623-661`), uniformly go to `Program { kind: Check }`                                                                                            |

**The duplication between `check_project` and `compile_project` (about 40% shared / 60% diverged).**
What's shared: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parse loop, per-file checker assembly, result handling. What's diverged:
items 2/3/4/5/7/8/9/10/11 in the 11-row table above. **That 40% shared skeleton is exactly
`Driver`'s value range** — all of the 60% divergence is "different stage set" or "different
aggregation mode", which is exactly what two `Program` fields can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design ruling: "upstream failure caused this stage to skip" is not a stage return value.**

Reason: skipping is **topology**-determined, not determined by the stage itself. If every stage
returned `Skipped`, then the information "why didn't I run" would be scattered across 12 stages with
no central audit. Instead, the dispatch loop decides at its head based on dependency relationships:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the bug this document is meant
to fix — the `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip" today; it produces
no diagnostic, just because the obligation happens to be empty. Once the rule is established, any
"didn't run Y because X didn't happen" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text Direction                                                                                                                                |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                              | "Stage Y was not executed because upstream stage X failed"                                                                                    |
| Caused by empty prerequisite obligation                 | "Stage Y was not executed because there were no pending obligations (normal)" — **this item's severity = Info, not counted in warning count** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y was not executed because the configuration is not enabled"                                                                           |

The second category is key: it lets "normal skip" and "abnormal skip" be distinguished in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count` (which
`diagnostic/mod.rs:637-641` depends on for non-blocking contract).

### 5. Diagnostic Aggregation: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                             | Current Correspondence |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (early-return at four places: `pipeline.rs:150/162/174/193`)    | Existing               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing               |

**`Aggregation` must be a field of `Program`, not a global Driver setting** — LSP serves both
in-project files and single files within the same process, while CLI's `run` and `check` are two
independent calls; putting them on the Program avoids the Driver holding mutable global state.

**Note the semantic difference between the two typecheck branches today**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) aren't just "early return or not" —
they are two different checker entries. After unification, the same implementation should be driven
by the `Aggregation` parameter, rather than keeping two functions (**this is one item this document
needs to additionally verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-Layer Impact                                                                                                                                        |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Doesn't introduce a new type representation, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                                |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only the ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                               |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what migrates into the ledger is the **settlement
responsibility**, not the type's owning module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, and the remaining field types each
stay in their original places; `driver/obligations.rs` only holds the aggregate container and
`assert_drained()`. L3 consumers (e.g. `ir_gen` reads `release_plan`) continue to receive specific
field types through parameters, and **must not `use crate::driver`** — this is consistent with the
driver-cannot-reverse-depend rule below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse-`use crate::driver`**. The
presence of `Driver` in `TypeChecker`'s imports is a violation, intercepted by the
`scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                     |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                             |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution is already plugged in by P3; **warning surface stays non-additive** (decision B1, 2026-10-09: dead-code-family warnings don't enter the compile path — pool semantics are being fixed by 4.9, normalizing the warning surface waits until after 4.9; the original draft's "new warning output" is voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                  |
| LSP (in-project)                    | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                  |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                    |

**The only intentional behavior break** is changing the `.expect()` at `predicate.rs:35` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract already declared at `backend.rs:60` ("caller should degrade conservatively") — the
current implementation contradicts its own contract documentation.

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

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                   |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                          |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` and used as `Stage::ProofExecution`'s implementation                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` thinned to Program constructor                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` thinned to Program constructor                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to go through Driver                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` driven by the `Aggregation` parameter                                                                                                                                                                                                                            |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics` manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                                 |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` uniformly goes to `Program { kind: Check }`                                                                                                                                                                                                                                                                              |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                         |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                          |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds a warning diagnostic                                                                                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix the comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer-order table changed to **actual** execution order; delete "if a lower layer fails, the upper layer is not run" (no short-circuit exists)                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access; the three consumers `predicate.rs`/`checker.rs`/`ownership.rs` go through the shared access. The bare `&'static` reference form is not viable (`dyn Solver` is not `Sync`); the original wording is governed by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse sequence
at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics are
themselves correct — **the problem is on the consumer side, not the producer side**, changing the
producer would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]`
branch (wasm branch reachability determination assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                          | Disposition                                                                                                                                      |
| --------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Multi-file path adds proof_execution                      | **breaking**: previously-silently-passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; staged release, expected diagnostic-set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **breaking**: programs that previously compiled now produce warnings   | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code unchanged                              |
| `assert_drained()` first run produces W-level diagnostics | **breaking**: diagnostic set grows                                     | W first, then E; S2/S4 in two steps                                                                                                              |
| `predicate.rs` panic changed to diagnostic                | **improvement**: no longer crashes                                     | No breakage                                                                                                                                      |
| `build` / `dump_bytecode` subcommands                     | **no impact**                                                          | These two paths don't go through proof_execution                                                                                                 |
| `TypeCheckResult` field migration to `Obligations`        | **internal refactor**                                                  | If `pub` API surface has external dependencies, sync them; in-repo consumers are all listed in the change list                                   |

---

## Implementation Points

This document corresponds to RFC-039's global stage sequence **P3 (fix correctness bugs)** and **P4
(stage contracts and unified Driver)**. S1-S5 below are the implementation order within this
document; **each step's acceptance criterion cites the C2 category of the
[Equivalence Oracle document](07-equivalence-oracle.md)** (orchestration change: same **diagnostic
set** across entries + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Bug criteria must be written as failing first.

| Deliverable                                                                                                                                              | Acceptance                                                                                         |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                            | **Must be red**. If it accidentally is green, this document's bug analysis needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48 — don't mix into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file corpora can be compared                              |
| `test_obligations_drained` skeleton                                                                                                                      | Mark `#[ignore]`, turn green in S4                                                                 |

**Rollback point**: no code changes, only new tests added.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-identical**                                |
| Full corpus (293 `.yx` files) diff                       | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately fixes no bug** — it only moves the existing behavior into the Driver. The
acceptance criterion is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, restore `pipeline.rs` as is.

### S3: Merge Four Orchestrator Entries + Fix Bugs

| Deliverable                                                                                                                     | Acceptance (C2)                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four-entry diagnostic set identical after `Aggregation` normalization               |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in-project and out-of-project                 |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                          |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Bug fixed                                                                           |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics without Z3                                                         |

**Rollback point**: bug fix and structural merge are **two separate commits**. If the structural
merge has issues, only the structural commit can be rolled back while retaining the bug-fix commit —
at which point `test_multifile_proof_obligation_not_dropped` stays green; the reverse (retain
structure, roll back fix) turns red, which is an unacceptable intermediate state, forbidden from
merging.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                          |
| -------------------------------------------- | --------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green              |
| `scripts/ci/check-obligations.py`            | Static gate effective                               |
| Obligation diagnostic W → E upgrade          | Manual review of every new E after full corpus diff |

**Rollback point**: `assert_drained()`'s severity is controllable via a config item; W/E switch
doesn't require code-structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                                     | Acceptance                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                          | Documentation corresponds item-by-item with `checker.rs` call sites                                                                                                                 |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown must not enter the cache** — otherwise a single timeout persists across compilations via the process-level cache and pollutes cargo test's inter-thread sharing) | Production-path `default_solver()` call sites reduced to zero (only tests retain them); cache hit rate observable across the three consumers (counters already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                                 | Has diagnostic when no `body_checker`                                                                                                                                               |
| Reachability determination of 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                           | Conclusion handed to `06-cleanup-inventory.md`; this document only registers the determination need                                                                                 |

**Note**: correcting the layer order will change the diagnostic set and may expose a flood of
previously-silent `Unproven`. **This item should walk independently from S1-S4**, not mixed into the
same PR as the entry merge.

## Key Decisions and Rationale

| Decision                 | Determination                                                                                                                                                         | Rationale                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustive enum, **rejecting runtime registration**                                                                              | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` won't compile. This is isomorphic to the mechanism the opcode table has already been validated as in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating the call-count questions that require human reasoning             |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, combined with the **static** gate in `scripts/ci/check-obligations.py`; severity W first, then E       | What we're fixing is a whole class of bugs, not one: the same kind of hidden risk has been verified in at least 2 places (`proof_calls`, `checker.rs:1313`), and all 16 span-keyed fields are in range. Going W first then E is to keep every step's C2 criterion usable — promoting to E in one step would make `yaoxiang check` suddenly flood with previously-silent diagnostics |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **don't touch** the producer-side logic of the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448` | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                              |

This design additionally closes two kinds of cracks:

- **The "comment promise" vs "code behavior" crack**. The "do not let silent pass return" at
  `checker.rs:5165` and "if a lower layer fails, the upper layer is not run" at `layers/README.md:3`
  are both **comment-level contracts** — the former is honored by `assert_drained()`, the latter is
  exposed by topology-driven `Skipped` diagnostics.
- **"Forgot to append to an array"**. Currently five functions each hand-write their own call
  sequence; after unification there is a single stage table, and any new stage missed will fail at
  compile time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — of the 11 inconsistencies, 7
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type interfaces**; they are "should we run some analysis"
  configuration questions; it also can't express `Aggregation`, and can only move the `check_module`
  / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, which is
  equivalent to swapping "five functions each hand-write the calls" for "an array forgot to append"
  — exactly the cause of the current 11 inconsistencies. Same-shape counterexamples already in the
  repo: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described
  in `docs/src/dev/design/check/` are all zero-implementation.
- **Only fix bugs without touching structure** — fixes the known 2 bugs, but only covers 1 of the 16
  obligation fields, `Stage` still scatters across 5 functions, and the next fork point will keep
  growing from here. **It must be done first** (part of S1/S3), because structural change needs a
  known-red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`** — `check_module` is a general entry; it
  doesn't know who the caller is, and `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the field is dropped **as a whole**. This is looking for a bug at
  the wrong level: the bug is in the orchestration layer, and the detection must be in the
  orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs` will be harder to read in the short term after thinning**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent problems"**. After the fix, there may be a wave of "new error" user reports —
  they are real existing bugs, they just never got reported. This must be clearly stated in
  CHANGELOG.
- **Severity switching in `assert_drained()` requires per-field human judgment**. Among the 16
  fields, some (e.g. `module_namespaces`) may be "no consumer needed" by design and should not be
  warned. S4 needs to walk through each field, not blanket-apply.
- **The `Program` abstraction may be premature**. If some entries' stage sets remain unstable in the
  long term, `stages()` degenerates to "a different array each call" as a free parameter, and the
  contract constraint falls through. **Mitigation: `test_program_stage_coverage` asserts that
  `stages()` can only come from the 6 predefined combinations.**
- **Criterion dependency**: the S1/S3 bug-fix criteria depend on the bug test being red first in the
  [Equivalence Oracle document](07-equivalence-oracle.md). **If that document doesn't first
  establish a multi-file corpus layer, S1 cannot be accepted** — because all 293 existing corpora
  take the single-file path, with zero coverage for this class of defect. This document doesn't
  involve the IR verifier (`verify_loose`); that preparatory work is out of scope.
- **Layer-order correction will expose previously-silent `Unproven`** (`equivalence` isn't even in
  the pipeline; `termination` and `ownership` order is opposite to the declaration). This is a
  diagnostic-set change risk and should walk independently.
- **Thread safety of the SMT backend after singleton conversion is undecided**. `backend.rs:13-19`
  already states `Solver` is only `Send`, not `Sync`, and `Z3Backend`'s cache is `RefCell`; after
  switching to a cross-compilation-unit shared singleton, all access paths must be confirmed to go
  through `Mutex`.
- **Stage parallelization is unevaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelism **masks order-dependency defects** (such as the actual order dependency between
  termination and ownership). This should be opened only after the equivalence criteria stabilize.

> **All open questions originally listed in this section have been ruled on.** See the per-item
> determinations in [RFC-039 Rulings](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — stage table and dependency direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactor (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` bug criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — design source for Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  comparison code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — explicit declaration of "do not let silent
  pass return"
- `src/frontend/core/typecheck/layers/README.md:3` — explicit declaration of "if a lower layer
  fails, the upper layer is not run" (no short-circuit actually exists)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — explicit rationale for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — contract declaration of "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that must be consumed
  downstream
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — single-test
  self-acknowledgment of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — corpus
  self-acknowledgment of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — the fourth compilation caller of the playground (single-file path, so
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate rather than main crate
- `build.rs:19-55` — error code build-time gate, an example of this project's enforcement mechanism
