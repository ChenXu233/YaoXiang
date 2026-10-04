# Compilation Stage Contract and Obligations Ledger

> **Subsidiary design document.** This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage sequence are covered in RFC-039
> itself; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction drawing for RFC-039. It addresses one
question: **"Which stages did this compilation run, which didn't, and why"** is currently scattered
across six paths and ten entry functions, each manually wired, with no single place that can answer
this question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture rather than logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` states verbatim "Unproven → compilation
error, no degradation, no silent pass……must not let silent pass resurrect." But this invariant only
holds on the **single-file path**: refinement constraints like `x: Sorted(3)` in proof functions
**silently pass** under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP—the
proof function is never executed, and no diagnostics are produced.

### Coverage

- `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and per-entry transformation for the ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers affected by the above changes, compatibility impact, and implementation
  order.

### Not covered

- Four-layer model, dependency direction conventions, anti-rebound gatekeeping → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 body

### Division of labor with RFC-039

RFC-039 gives the **why** of the refactor and the **order**; this document gives the **specific
form** of L1, the **per-file change list**, and the **internal implementation stages**
(corresponding to RFC-039's global sequence P3 "fix correctness vulnerabilities" and P4 "stage
contract and unified Driver"). In case of conflicts between this document and RFC-039, RFC-039
prevails.

Equivalence criteria are executed per the **C2 (orchestration change)** category of the
[equivalence criteria document](07-equivalence-oracle.md): same diagnostic set for each entry + same
corpus behavior.

## Current State

> **All facts in this section are verified**, each with file path + line number. Line numbers are
> based on `9e02e4db`.

### Stage boundaries are the unique "write it wrong in one place, silent everywhere" structural defect

It differs in nature from the other two categories of defects (module boundaries, test wiring):

| Defect                  | Typical manifestation            | Is there a signal?   |
| ----------------------- | -------------------------------- | -------------------- |
| Lexical/syntactic error | Wrong source code                | Has diagnostic       |
| Type mismatch           | Wrong type                       | Has diagnostic       |
| Stage miss-wiring       | A stage is never called          | **No signal at all** |
| Obligation not consumed | Field filled but no one reads it | **No signal at all** |

The common characteristic of the latter two categories is: **failure produces no error**. Therefore,
they cannot be solved by "writing code more carefully" or "reviewing more strictly"—code review can
only see what was written, not **what wasn't written**. This is exactly the root cause diagnosed by
RFC-039: "this project treats 'design' as a documentation convention rather than an executable
constraint."

### Enforcement mechanisms already present in the repository

The same repository already has mature enforcement mechanisms, they just haven't been extended to
the stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subject to **build-time
  hard gatekeeping** by `build.rs:19-55` via `tools/code-tables`, comparing one by one against the
  RFC-013 code table, refusing to compile on any inconsistency with `panic!`.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` directory has 6227 lines, 47.9%) has
  high test density, with every module's test subtree being explicitly wired—this is the most
  complete test wiring in the repository, serving as a formal reference for stage-layer gatekeeping.

**Design capability is sufficient. What's missing is "putting an equally-tiered gatekeeper in the
orchestration layer as well."**

(Line count convention: `(Get-Content).Count`, see the "Line Count Convention" section in
`06-cleanup-inventory.md`.)

### Six compilation paths, ten entry functions

| #   | Path                       | Entry function                                        | Location                                      | Which compilation path                         |
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

| Stage               | Call site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)— RFC-027 Phase 2.5                  |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 instances of stage coverage inconsistency

The table below provides evidence cell by cell. **Empty cells do not mean "this entry doesn't do
this", but rather "no code in this entry does this"**—this is the essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **No**                                                    | **No**                                                      | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **No**                                                    | Yes `356-377` (role-aware, no config gating)                | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                           | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                            | Yes `350`                                                   | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                              | `discover` (`101`, drops `used_by`/`shadow_events`)       | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                           | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | N/A                                              | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                      | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (independent ModuleIR then merge) |

**Two places require precise description, otherwise it will be written wrong:**

- **The precise conclusion of rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138`—`type_results.push(result)` stores the full `TypeCheckResult` (including
  `warnings`), but this function **has no `result.warnings` read site at all**, and `result`'s only
  downstream use is `generate_ir_with_context` (`154-156`).
- **Row 11's entry `main` criterion also has a different source**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surfaces); `compile_project` uses `is_bin_role` (`250-252`,
  only judges "has manifest or not"). The two functions give different answers to "what files must
  define `main`."

### Correctness vulnerability: proof obligations silently discarded (complete evidence chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the entire
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

The semantics are: "the predicate arguments of this refinement constraint are compile-time literals,
and the proof function must actually be executed to determine it."

**Step 2 — The consumption point is inside checker, but it only emits, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real defense line rather than a forward-looking backstop: must not let silent pass resurrect.`

That is: the author knew that "not producing diagnostics" is a defect, explicitly classified it as
**a known intermediate state with nowhere to go**, and assumed in the design that "someone will
definitely come read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares the local
`proof_calls` and collects, `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) has only **one read site in production code at `src/frontend/pipeline.rs:187`**
(`189` passes the argument):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other occurrences of the `proof_calls` identifier in the repository fall into three categories,
none of which is a consumer of this field: `checker.rs:1234/4564/5494` is the production-side
collection; `verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads
`ProofResult`.)

**Step 5 — The orchestrator's four entries all bypass that layer.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), `compile_embedded_module` (`1374`) **all do not call
`pipeline.rs`**, directly calling `TypeChecker::check_module` (`124` / `493` / `1386`). Therefore,
they don't even reach that sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: even the standard library's own refinement obligations go through the
discard path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std module itself are
also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP three paths **compiles, runs,
produces no diagnostics**. The proof function is never called.

**Step 8 (supplementary verification) — There's a second silent discard point within the layer.**
`checker.rs:1303-1314` when handling the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic, no
accounting.** This directly contradicts the `5164` branch's promise of "must not let silent pass
resurrect", and is **independent of the orchestrator issue**—even if the entry layer is completely
fixed, this one will still be silent. (Handling timing: should be processed in the same batch as the
obligations ledger, because it's the same kind of issue as the obligations ledger.)

### Why tests didn't catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping right before `pipeline.rs`**.

The test file's own documentation comment already states this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer.`

**This is exactly the shape of the problem: tests verify "the obligation is filled", while the bug
is "the consumer didn't read it".** A test that only tests the production side and not the consumer
side is naturally immune to this class of defect.

### Stronger finding: the equivalence criteria's main corpus is the single-file path

The [equivalence criteria document](07-equivalence-oracle.md) uses end-to-end differential of 293
`.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion**, the main acceptance method for
C2 stage. But actual testing shows:

- There is **no `yaoxiang.toml`** anywhere under the `tests/` directory (zero hits with directory
  glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is empirical evidence
of this. That file is marked `// expect: compile-error E4018` (line 15), and its header comment on
line 7 reads "status: ❌ should be rejected at compile time"—**it can pass precisely because it goes
through the only path that executes proof functions**.

**Conclusion: the third-layer criterion needs to supplement the multi-file corpus layer, otherwise
it cannot serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation fields: 16 "produce-as-contract" fields, zero mechanism guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields** whose
documentation comments explicitly state "produced by stage X → consumed by stage Y", meaning they
are essentially **cross-stage obligations**:

| Field                      | Definition line | Producer → Consumer                                             |
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
criteria document has already pointed out that mismatches in span-keyed contracts "fail silently
with no error" (typical of `ReleasePlan`: any inconsistency in span calculation on either side →
`Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- `release_plan`'s consumer `ir_gen.rs:341` (`release_plan: type_result.release_plan.drops.clone()`)
  is inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in a downstream module, and the downstream module is on
  the required path for all entries.
- `proof_calls`'s consumer is at the **top level** of `pipeline.rs`, belonging to **another entry
  implementation**. The orchestrator's four entries don't go through `pipeline.rs` at all.

**Verified fact**: currently the entire repository has **no mechanism** to guarantee that these 16
fields are consumed. The only "protection" is that `ReleasePlan` happened to ride along with IR
generation.

### Four additional contract defects in the proof layer

All below come from proof layer analysis, independent of the entry forking issue, but all belong to
the category of "stage contract not enforced".

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 claims "execute in layer order, upper layer doesn't run when lower layer fails".
Actual call sites in `TypeChecker::check_module`:

| Actual order | Call site                                         | Corresponding declared layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: `termination` and `ownership` order is **opposite** to the declaration; the declared Layer
0 `equivalence` **does not appear in `check_module`'s stage sequence at all** (it's only used as an
`is_subtype` utility function by `inference/assignment.rs:17`, unrelated to `ProofResult`). And
**there is no short-circuit**—`checker.rs:1271-1276` adds termination errors one by one via
`add_error` and continues, the README's promise of "upper layer doesn't run when lower layer fails"
doesn't hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

| Consumer                                                | Acquisition strategy                                     | When solver unavailable                                   |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Construct-time injection `with_solver_owned`             | `None` → **silently skip** (don't inject)                 |
| `ownership.rs:627` (back-edge cut determination)        | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively don't cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "initialization failure
maintains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds
kernel capability'". While `backend.rs:60`'s contract documentation says " `None`: backend
unavailable……**caller should conservatively degrade**". **Two philosophies coexist in the same
codebase, and the hard-fail one panics rather than returning an error.**

**(c) Each back-edge determination creates a new Z3 context, cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut determination. While
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—each call does `Z3Backend::new()`, which creates a
new Z3 context. The `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **intra-instance**, and the documentation comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits in this call mode**.

(For comparison: `predicate.rs:34-36`'s `LazyLock` is indeed a singleton. Same backend, two
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

When `body_checker` is `None`, the ownership check receives **an empty type ledger and an empty call
table** and runs as usual—ownership analysis degrades to "nothing conflicts", **without any
warning**. (Suggested handling: should be recorded as a warning-level diagnostic, or at least leave
a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking, before constraint
solving". Actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination is at `1256`,
ownership at `1296`, i.e. what the comment calls "before" is actually "after two layers"**.

### wasm current state: carried by shim crate, 27 files' branches are alive

The wasm target **is built and built in CI**, with `cdylib` not in the main crate but in a separate
shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                        |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                              |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                              |
| shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                     |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                 |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                        |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten  | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from fixed URL, degrade to warning if missing)                                                                                                                                                                |
| **27 files** contain the `wasm32` literal         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in documentation                                                                                                                            |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                      |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                      |

**Conclusion: these 27 files' `#[cfg(target_arch = "wasm32")]` branches are load-bearing.** They
determine which APIs the main crate can be called from `wasm/src/lib.rs` (73 lines) under the wasm
target—`lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` entirely (these
three all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly related to this document**: `wasm/src/lib.rs:48`'s playground
entry calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Paths that consume `proof_calls`           | Whether proof function executes |
| ------------------------------------------ | ------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                         |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                         |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                         |
| Multi-file `run` (`compile_project`)       | **No**                          |
| `check` (`check_project`)                  | **No**                          |
| LSP in-project (`check_source_in_project`) | **No**                          |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see "Target Design" 3), otherwise unifying the Driver would miss this
path.

(Cleanup scope—which of `orchestrator.rs`'s 12 wasm attributes are unreachable in the playground
scenario—goes to `06-cleanup-inventory.md`. The open question in RFC-039 about "build wasm target,
or delete these branches" **is now obsolete**: the target is built.)

---

## Target Design

### 1. Stage model: `Stage` enum (exhaustive, no runtime registration)

**Core constraint: `Stage` is a compile-time exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer"—this is exactly the mechanism proven effective by the opcode
table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Parsing,            // Lex + syntax                     PerModule
    Typecheck,          // Type check (with embedded proof layers)   PerModule
    DeadCodeAnalysis,   // Dead code family analysis        Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    Monomorphization,   // Monomorphization                Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Linking,            // Cross-module linking / IR merge      Project
}

impl Stage {
    /// All stages. When adding new variants, this array and `dispatch`'s exhaustive match will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (once per compilation unit) and `Project` (once at the
project level) turn "which stages must run per module, which must run once at the project level"
into a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once in
`dispatch`—this eliminates questions like "should `allocate_global_slots` be called for each file"
in current `orchestrator.rs` that require human reasoning.

**The stage table is in topological order, not alphabetical**, because failure propagation depends
on order.

### 2. Obligations ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no consumer" from undetectable to a compile-time or
test-time assertable fact. RFC-039 has already listed this as "the highest-value item."

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
    // …other span-keyed fields
}

impl Obligations {
    /// Settlement: call once at the end of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                          | Disposition                                                           |
| ------------------------------------ | -------------------------------------------------- | --------------------------------------------------------------------- |
| Obligation read by declared consumer | Field in `Obligations` is `take()`/marked consumed | Pass                                                                  |
| Obligation non-empty but no consumer | Field is non-empty and not consumed                | **Produce diagnostic** (W-level by default; E-level in `strict` mode) |
| Obligation empty                     | Field is empty                                     | Pass (no consumption needed)                                          |

**Why start with W-level instead of E-level**: fixing obligation discarding will **change the
diagnostic set**. The C2 criterion requires "same diagnostic set for each entry", if jumping
straight to E-level, `yaoxiang check` will suddenly have lots of previously-silent `Unproven`
diagnostics. Two steps (observe with W first, then upgrade to E) allow every step's criterion to be
usable. See "Implementation Points" S4 for details.

**`assert_drained()` has only one call site**: `Driver::run` at the end of the stage table, before
producing `CompilationResult`.

**Companion static gatekeeping**: RFC-039 has already proposed `scripts/ci/check-obligations.py`
(field appears ≥2 times but no third file reads it → fail). `assert_drained()` is a **runtime**
gate, the script is a **static** gate, both are complementary, both are needed.

### 3. Unified Driver: single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — fourth caller of single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be the above 6 predefined combinations (one for each `ProgramKind`), does not
accept caller passing arrays freely**. This is the key constraint preventing the `Program`
abstraction from degrading into "free parameter that accepts anything", asserted by
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
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one less
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Only settlement point
        Ok(state.into_result())
    }
}
```

**Transformation for ten entries (specify function per entry)**:

| Entry                                                      | After transformation                                                                                                                                                                                            |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                  |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already stateless wrapper)                                                                                                                                                                           |
| `compile_project` (`orchestrator.rs:99-237`)               | Thin down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                   |
| `check_project` (`orchestrator.rs:273-399`)                | Thin down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                     |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filtering to target file                                                                                                                                                     |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                       |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (forward to `run_project` or `SingleFile`)                                                                                                                                                            |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (forward to Driver)                                                                                                                                                                                   |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project goes to `Program { kind: Lsp, aggregation: CollectAll }`; **delete** the manual lex→parse→… sequence in the single-file branch, call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                   |

**Repeat between `check_project` and `compile_project` (about 40% shared / 60% diverged)**. Shared:
vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`, parsing
loop, per-file checker assembly, result handling. Diverged: items 2/3/4/5/7/8/9/10/11 in the 11-item
table above. **This 40% shared skeleton is exactly the value range of `Driver`**—the 60% diverged is
all "different stage sets" or "different aggregation modes", which are exactly what `Program`'s two
fields can completely express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causes this stage to skip" is not a stage return value.**

Reason: skip is determined by **topology**, not by the stage itself. If each stage returns `Skipped`
itself, then the "why didn't I run" information is scattered across 12 stages and cannot be
centrally audited. Instead, have `dispatch` determine it at the beginning of the loop based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
aims to fix—current `pipeline.rs:187`'s `if !proof_calls.is_empty()` is a silent "skip", it produces
no diagnostic, just because the obligation happens to be empty. After the rule is established, any
"because X didn't happen so Y didn't run" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                    | Text direction                                                                                                               |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                                | "Stage Y was not executed due to failure of upstream stage X"                                                                |
| Caused by empty prerequisite obligation                   | "Stage Y was not executed due to no pending obligation (normal)"—**this item severity = Info, not counted in warning count** |
| Conditions not met (e.g., `config.mono.enabled == false`) | "Stage Y was not executed due to configuration not enabled"                                                                  |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` relies on this count's non-blocking contract).

### 5. Diagnostic aggregation patterns: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                            | Current correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)              | Exists                 |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists                 |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
separate invocations; putting them on Program avoids Driver holding mutable global state.

**Note the current semantic difference between the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of "early
exit or not"—they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter, rather than retaining two functions (**this is an
item that needs additional verification, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type system impact

| Change                                      | Type layer impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation introduced, no touch on `MonoType` / `PolyType` / `ir::Type`                                                    |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to language type system                                                                                               |

**This document does not introduce a fourth set of type representations.** The convergence of three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Belonging and layering of obligation types**: What's migrated to the ledger is **settlement
responsibility**, not type ownership module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay in their
original places; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`.
L3 consumers (e.g., `ir_gen` reads `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`**—this is consistent with the driver's
no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**.
`Driver` appearing in `TypeChecker`'s import is a violation, intercepted by the
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime behavior

| Scenario                            | Before transformation                    | After transformation                                                                      |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                             |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | **New** proof_execution; **new** warning output                                           |
| `yaoxiang check` (multi-file)       | No proof_execution                       | **New** proof_execution                                                                   |
| LSP (in-project)                    | No proof_execution                       | **New** proof_execution                                                                   |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**) |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                    |

**The only intentional behavior break** is `predicate.rs:35`'s `.expect()` changed to return
diagnostic. This conforms to RFC-027 §8's positioning of "not bound to specific solver", and is also
the contract already declared by `backend.rs:60` ("caller should conservatively degrade")—current
implementation is the opposite of its own contract documentation.

### Compiler change list

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

| File                                               | Line range                         | Change                                                                                                                                                         |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                          |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                 |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                               |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                               |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                         |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` as the implementation of `Stage::ProofExecution`                                                       |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` thinned down to Program constructor                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` thinned down to Program constructor                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` switched to Driver                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` driven by `Aggregation` parameter                                       |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, switched to Driver call                                                                                     |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                            |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`) |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fixes second silent discard point**)                                           |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment that doesn't match `1320`                                                                                                                      |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; remove "lower layer fails, upper layer doesn't run" (no short-circuit)                                |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | `default_solver()` changed to return `&'static` singleton reference (fixes per-back-edge new context)                                                          |

**Deleted**: `src/util/diagnostic/mod.rs:621-661`'s `check_single_file`;
`src/lsp/handlers/diagnostics.rs:161-227`'s manual lex/parse sequence.

**Not touched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct—**the problem is on the consumer side, not the producer side**, changing the
producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branches (reachability judgment of wasm branches goes to
`06-cleanup-inventory.md`).

### Backward compatibility

| Change                                                    | Compatibility                                                          | Disposition                                                                                                                                          |
| --------------------------------------------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously-silently-passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compiled now produce warnings              | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                                |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set increases                                 | W first then E, S2/S4 in two steps                                                                                                                   |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                     | No breaking                                                                                                                                          |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                          | These two paths don't go through proof_execution                                                                                                     |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactor**                                                  | If `pub` API surface has external dependencies, need to sync; in-repository consumers are fully listed in the change list                            |

---

## Implementation Points

This document corresponds to **P3 (fix correctness vulnerabilities)** and **P4 (stage contract and
unified Driver)** of RFC-039's global stage sequence. The following S1-S5 are the internal
implementation order of this document, **each step's acceptance criteria references the C2 category
of the [equivalence criteria document](07-equivalence-oracle.md)** (orchestration change: same
**diagnostic set** for each entry + same corpus behavior).

### S1: Establish criteria (red first)

**Prerequisite, cannot be skipped.** The vulnerability criteria must first be written as failing.

| Deliverable                                                                                                                                             | Acceptance                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                           | **Must be red**. If accidentally green, the vulnerability analysis of this document needs to be re-reviewed |
| Multi-file corpus layer: new `tests/yaoxiang-multifile/` (Resolution D48—not mixed into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file versions can be compared                                      |
| `test_obligations_drained` skeleton                                                                                                                     | Marked `#[ignore]`, S4 turns green                                                                          |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, don't change behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-identical**                             |
| All corpus (293 `.yx`) diff                              | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately does not fix any bugs**—it only moves existing behavior into Driver.
Acceptance criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` can be restored to original.

### S3: Merge orchestrator four entries + fix vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four-entry diagnostic sets normalized by `Aggregation` are identical              |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` has consistent behavior inside and outside the project           |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI produce same diagnostic set for the same file                         |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics without Z3 environment                                           |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If
structural merge has issues, can only roll back the structural commit, keep the vulnerability fix
commit—at this point `test_multifile_proof_obligation_not_dropped` stays green; reverse (keep
structure, roll back fix) will turn red, which is an unacceptable intermediate state, forbidden to
merge.

### S4: Enable obligations ledger

| Deliverable                                  | Acceptance                                          |
| -------------------------------------------- | --------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green              |
| `scripts/ci/check-obligations.py`            | Static gatekeeping effective                        |
| Obligation diagnostic W → E upgrade          | After full corpus diff, manual review of each new E |

**Rollback point**: severity of `assert_drained()` can be controlled by configuration, W/E switch
doesn't need to change code structure.

### S5: Proof layer and wasm wrap-up

| Deliverable                                                     | Acceptance                                                                                          |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order          | Documentation corresponds one by one with `checker.rs` call sites                                   |
| `backend.rs:67-72` changed to singleton                         | Cache hit rate observable (add counter)                                                             |
| `checker.rs:1283-1293` degradation adds warning                 | Diagnostic exists when no `body_checker`                                                            |
| Reachability judgment of `orchestrator.rs`'s 12 wasm attributes | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the judgment requirement |

**Note**: correcting the layer order will change the diagnostic set, possibly exposing a large
number of previously-silent `Unproven`. **This item should be processed independently from S1-S4**,
not mixed in the same PR as entry merging.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                       | Reason                                                                                                                                                                                                                                                                                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustive enum, **runtime registration rejected**                                                                             | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant, `dispatch` won't compile. This is isomorphic to the mechanism validated in RFC-039 routing table B by the opcode table. `StageScope` further turns "per module or project-level" into a type fact, eliminating call-count issues that require human reasoning    |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, combined with **static** gatekeeping from `scripts/ci/check-obligations.py`; severity W first then E | What it fixes is an entire class of bugs, not one: there are at least 2 verified similar hidden dangers (`proof_calls`, `checker.rs:1313`), and all 16 span-keyed fields are in range. W first then E is to make every step's C2 criterion usable—jumping to E at once will make `yaoxiang check` suddenly have lots of previously-silent diagnostics |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **don't touch** the six `Unproven` branch producer-side logics at `checker.rs:5164/5179/5306/5318/5420/5448`       | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                |

This design additionally eliminates two types of cracks:

- **The crack between "comment promises" and "code behavior"**. The "must not let silent pass
  resurrect" at `checker.rs:5165` and the "lower layer fails, upper layer doesn't run" at
  `layers/README.md:3` are both **comment-level contracts**—the former is fulfilled by
  `assert_drained()`, the latter is exposed by topology-driven `Skipped` diagnostics.
- **"Forgot to append in one array"**. Currently five functions each hand-write the call sequence;
  after unification, there's only one stage table, and missing a new stage will cause compile-time
  error.

### Not adopted directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—Of the 11 inconsistencies, 7
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, they're configuration questions of "should I
  run some analysis"; it also can't express `Aggregation`, only moving the choice between
  `check_module` / `check_module_collect_all` from function layer to type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—Eliminates compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, equivalent to
  changing "five functions each hand-write calls" to "one array forgot to append", which is exactly
  the cause of the current 11 inconsistencies. Same-form counterexample in the repository: the
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/design/check/` are all zero-implemented.
- **Only patch bugs without touching structure**—Can fix the known 2 bugs, but only covers 1 of 16
  obligation fields, `Stage` still scattered in 5 functions, the next fork point will continue to
  grow from here. **It must be done first** (it's part of S1/S3), because structural transformation
  needs a known red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` can't hold; `#[must_use]`
  only warns when the field is **wholly** discarded. This is looking for the bug at the wrong level:
  the bug is in the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs` thinning down will make it harder to read in the short term**. After 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize wiring" refactors.
- **S3 will significantly change the diagnostic set, in the direction of "exposing previously-silent
  problems"**. After the fix, there may be a wave of "new errors" user reports—they're real bugs,
  just never reported before. Must be clearly stated in CHANGELOG.
- **Severity switch of `assert_drained()` needs per-field manual judgment**. Among the 16 fields,
  some (like `module_namespaces`) having "no consumer" might be by design not needing consumption,
  shouldn't be alarmed. S4 needs to go through each field, can't do a blanket approach.
- **`Program` abstraction may be premature**. If some entries' stage sets are long-term unstable,
  `stages()` will degrade to "different array each call" free parameter, contract constraint fails.
  **Mitigation is `test_program_stage_coverage` asserts that `stages()` can only come from 6
  predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criteria depend on the vulnerability test from
  the [equivalence criteria document](07-equivalence-oracle.md) being red first. **If that document
  doesn't first establish the multi-file corpus layer, S1 cannot be accepted**—because all existing
  293 corpora go through the single-file path, with zero coverage of this class of defect. This
  document doesn't involve the IR verifier (`verify_loose`), that part of the prerequisite work is
  not in this document's scope.
- **Layer order correction will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order is opposite to declaration). This is a
  diagnostic set change risk, should be processed independently.
- **Thread safety after SMT backend change to singleton is undecided**. `backend.rs:13-19` already
  states `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after changing to
  cross-compilation-unit shared singleton, need to confirm all access paths go through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order dependency defects** (e.g., actual order dependency between
  termination and ownership). Should be opened after equivalence criteria are stable.

> **All open questions originally listed in this section have been decided.** Per-item decisions are
> in [RFC-039 Decision Register](../rfc/draft/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../rfc/draft/039-compiler-architecture.md) — Four-layer
  model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction conventions,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` transformation (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../rfc/accepted/013-error-code-specification.md) —
  `build.rs:19-55` gate's comparison code table
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "must not let silent
  pass resurrect"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "lower layer fails,
  upper layer doesn't run" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that must be consumed
  downstream
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus
  self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Fourth compilation caller of the playground (single-file path, thus
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate rather than main crate
- `build.rs:19-55` — Error code build-time gate, an example of forced mechanism in this project
