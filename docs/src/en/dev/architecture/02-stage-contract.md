# Compilation Stage Contract and Obligations Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution stage order are in the RFC-039 main
> text; the positioning of each companion document is in [this directory's index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint of RFC-039. It addresses a
single question: **"Which stages ran in this compilation, which did not, and why"** is currently
scattered across six paths and ten entry functions, each wired up by hand, with no single place that
can answer it.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are violated by the architecture, not by the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally says "Unproven → compile
error, no degradation, no silent pass... must not let silent pass come back." But this invariant
only holds on the **single-file path**: proof function constraints like `x: Sorted(3)` **silently
pass** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths, proof functions are never
executed, and no diagnostics are produced.

### Coverage

- The `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry migration plan for all ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of scope

- Four-layer model, dependency direction specification, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1-C6 tiers, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- Global execution order P1-P10 and acceptance gates G1-G10 → RFC-039 main text

### Division of labor with RFC-039

RFC-039 gives the **why** for the refactor and the **order** to do it; this document gives L1's
**concrete shape**, the **per-file change list**, and the **internal implementation stages of this
document** (corresponding to RFC-039's global sequence P3 "fix correctness vulnerabilities" and P4
"stage contract and unified Driver"). Where this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria follow the **C2 (orchestration changes)** category of the
[Equivalence Oracle document](07-equivalence-oracle.md): identical diagnostic sets across entries +
identical corpus behavior.

## Current State

> This section is entirely **verified facts**, each with a file path + line number. Line numbers are
> based on `9e02e4db`.

### Stage boundaries are the only structural defect where "one place wrong, the whole repo silent"

It differs in nature from the other two defect types (module boundaries, test wiring):

| Defect                   | Typical manifestation               | Any signal?          |
| ------------------------ | ----------------------------------- | -------------------- |
| Lexical/syntactic errors | Source code written incorrectly     | Has diagnostic       |
| Type mismatch            | Type written incorrectly            | Has diagnostic       |
| Stage skipped wiring     | A stage is simply never called      | **No signal at all** |
| Obligation not consumed  | Field filled in but no one reads it | **No signal at all** |

The common characteristic of the latter two is: **failure produces no error**. Therefore they cannot
be solved by "writing code more carefully" or "reviewing more strictly" — code review can only see
what was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"this project treats 'design' as a documentation convention rather than an executable constraint."

### The repository already has mature enforcement mechanisms

The same repository already has mature enforcement mechanisms — they just haven't been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, comparing one-by-one against
  the RFC-013 code table; any inconsistency causes a direct `panic!` rejecting compilation.
- `src/package/` (**76 files / 13,012 lines**, of which 6,227 lines and 47.9% are in `tests/`) has
  high test density, and each module's test subtree is wired up declaratively — this is the most
  completely wired-up block in the repository, and can serve as the formal reference for stage-layer
  gates.

**The design capacity is sufficient. What's missing is "putting a same-level gate at the
orchestration layer too."**

(Line count metric: `(Get-Content).Count`, see the "line count metric" section in
`06-cleanup-inventory.md`.)

### Six compilation paths, ten entry functions

| #   | Path                       | Entry function                                        | Location                                      | Which compilation                               |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                            |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking          |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics     |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
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
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded in it** (`381-389`) |

### 11 stage coverage inconsistencies

The table below gives evidence cell by cell. **A blank does not mean the entry "doesn't do this" —
it means "no code in that entry does this"** — which is the very nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                    | **No**                                                      | **No**                             | **No**                                 |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                    | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                                 |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                 |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)         | **Collected but never output**                            | Yes `350`                                                   | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, drops `used_by`/`shadow_events`)       | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                 |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`205`)                                         | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (after independent ModuleIR merge) |

**Two places need precise wording, or you'll get it wrong:**

- **The accurate conclusion for rows 4/5**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is in
  `compile_project:138` — `type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function **has no `result.warnings` read site throughout**, and
  the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion also differs in source across entries**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surfaces); `compile_project` uses `is_bin_role` (`250-252`,
  only checks "has manifest or not"). The two functions give different answers for "which files must
  define `main`".

### Correctness vulnerability: proof obligations silently dropped (complete evidence chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation has a single production point.**
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

The semantics is "the predicate argument of this refinement constraint is a compile-time literal,
and this proof function must actually be executed to decide."

**Step 2 — The consumption point is inside the checker, but it only produces without consuming.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                      | Location                                                                                                     | Behavior                |
| --------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parametric refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check    | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation  | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ...
> `This branch therefore becomes a real line of defense rather than forward-looking prepayment: must not let silent pass come back.`

That is: the author knew "producing no diagnostic" was a defect, explicitly classified it as a
**known, nowhere-to-go intermediate state**, and designed under the assumption that "someone will
definitely come and read `proof_calls`."

**Step 3 — That field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects, `checker.rs:1439` writes it as
`proof_calls, // Phase 2.5 pre-registered proof function obligations` into `TypeCheckResult`. The
field is defined at `types.rs:29`.

**Step 4 — The repository has only one read site.** `TypeCheckResult.proof_calls` (`types.rs:29`) in
production code is **only read at one place: `src/frontend/pipeline.rs:187`** (passed as argument at
`189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repository fall into three categories, none
of which are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries bypass that layer.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), `compile_embedded_module` (`1374`) **all do not call
`pipeline.rs`**, calling `TypeChecker::check_module` directly (`124` / `493` / `1386`). So they
don't even reach the only read site at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations also take the drop path.**
`compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for compiling the
embedded std. This means **the proof obligations of the embedded std module itself are likewise not
executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, and emits no diagnostic at all** on the
multi-file / check / LSP three paths. The proof function is never called.

**Step 8 (supplementary verification) — The layer has a second silent drop point.**
`checker.rs:1303-1314` handling the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and
no accounting.** This directly contradicts the `5164` branch's promise of "must not let silent pass
come back", and is **independent of the orchestrator issue** — even if the entry layer is completely
fixed, this one will remain silent. (Processing timing: should be handled in the same batch as the
obligations ledger, because it is the same kind of problem as the obligations ledger.)

### Why tests didn't catch this

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example `rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call
exists in `result.proof_calls`. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping exactly before `pipeline.rs`**.

The test file's own documentation comment has already written this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer`

**This is exactly the shape of the problem: tests verify "obligations are filled in", but the bug is
"the consumer doesn't read them."** A test that only tests the production side and not the consumer
side is naturally immune to this kind of defect.

### A stronger finding: the equivalence oracle's main corpus is the single-file path

The [Equivalence Oracle document](07-equivalence-oracle.md) uses the end-to-end diff of 293 `.yx`
corpus files in `tests/yaoxiang/` as the **third-layer criterion** and the main acceptance means for
the C2 stage. But actual testing shows:

- There is **no `yaoxiang.toml`** in the `tests/` directory at all (zero hits in the entire
  directory glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) → `check_single_file` (`623-661`) → `Compiler::compile_with_source` (`635`) →
  `Pipeline::run` → **proof_execution runs** for each corpus file.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  and none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is precisely the
empirical evidence. This file is tagged `// expect: compile-error E4018` (line 15), and its header
comment at line 7 says "status: ❌ should be rejected at compile time" — **it passes, precisely
because it takes the only path that executes the proof function**.

**Conclusion: the third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as this document's acceptance tool.** See "Implementation Notes" S1.

### Obligation fields: 16 "produce-is-contract" fields, zero mechanism guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields** whose
doc comments explicitly state "produced by stage X → consumed by stage Y", i.e., they are
essentially **cross-stage obligations**:

| Field                      | Definition line | Producer → consumer                                             |
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

**All these fields are passed across layers keyed by `Span` or name table.** The equivalence oracle
document has already pointed out that span-keyed contract mismatch will "silently fail, with no
error" (`ReleasePlan` is typical: any inconsistency in how the two sides compute span → `Drop`
instructions silently disappear).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), is inside `generate_ir_with_context`,
  and `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the
  consumer is in a downstream module, and the downstream module is on the must-pass path of all
  entries.
- The consumer of `proof_calls` is at the **top of `pipeline.rs`**, which belongs to **another
  entry's implementation**. The orchestrator's four entries don't go through the `pipeline.rs` layer
  at all.

**Verified fact**: currently the repository **has no mechanism** that guarantees these 16 fields are
consumed. The only "protection" is that `ReleasePlan` happened to catch a ride on the IR generation
bus.

### Four additional contract defects in the proof layer

The following all come from proof-layer analysis, are independent of the entry-fanout problem, but
all belong to the category "stage contract is not enforced."

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Depends on    |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "execute in layer order, lower-layer failure prevents upper-layer from running."
The actual call sites inside `TypeChecker::check_module`:

| Actual order | Call site                                         | Corresponding declared layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: `termination` and `ownership` are in **reverse** order from the declaration; the declared
Layer 0 `equivalence` **does not appear at all in `check_module`'s stage sequence** (it is only used
by `inference/assignment.rs:17` as an `is_subtype` utility, unrelated to `ProofResult`). At the same
time, **there is no short-circuit at all** — `checker.rs:1271-1276` adds termination's errors
one-by-one `add_error` and then continues, so README's promise of "lower-layer failure prevents
upper-layer from running" doesn't hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

| Consumer                                                | Acquisition strategy                                     | When solver unavailable                                   |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Injection at construction time via `with_solver_owned`   | `None` → **silently skip** (not injected)                 |
| `ownership.rs:627` (back-edge cut-off determination)    | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "initialization failure
maintains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond
kernel capability'." While `backend.rs:60`'s contract document says "`None`: backend unavailable...
**the caller should conservatively degrade**". **Two philosophies coexist in the same codebase, and
the hard-failure one panics rather than returning an error.**

**(c) Each back-edge determination creates a new Z3 context, cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut-off determination.
While `src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — every call does `Z3Backend::new()`, i.e., creates
a new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **per-instance**, while `z3_backend.rs:17`'s doc
comment claims "SMT query results are cached in `cache`." Not shared across calls ⇒ **this cache
never hits under this call pattern**.

(For reference: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifetime strategies.)

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

When `body_checker` is `None`, ownership check gets an **empty type ledger and empty call table**
and runs anyway — ownership analysis degenerates to "nothing conflicts," **with no warning**.
(Processing suggestion: should be recorded as a warning-level diagnostic, or at least left as a
trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving." Actually `self.env.solver().solve()` is at `checker.rs:1320` — **termination is
at `1256` and ownership at `1296` after that, i.e., what the comment says is "before" is actually
"two layers after"**.

### wasm current state: shim crate carries it, 27 files' branches are alive

The wasm target **is built and built in CI**, the `cdylib` is not in the main crate but in an
independent shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                               |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                               |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                      |
| Main crate has wasm target dependencies section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                  |
| Shim directory is excluded from main workspace    | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                         |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from a fixed URL, degrade to warning on missing)                                                                                                                                                               |
| **27 files** have the `wasm32` literal            | Of which **25** have actual `#[cfg(...)]` attributes, another 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are documentation mentions only                                                                                                                                   |
| `orchestrator.rs` 20 places                       | 12 attributes + 8 comments                                                                                                                                                                                                                                                       |
| `lib.rs` 11 places                                | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate the 73-line `wasm/src/lib.rs` can call under the wasm
target — `lib.rs:139/153/170` gates the entirety of `run_file` / `run_project` / `build_bytecode`
(all three require `std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Path consuming `proof_calls`               | Whether it executes proof function |
| ------------------------------------------ | ---------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                            |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                            |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                            |
| Multi-file `run` (`compile_project`)       | **No**                             |
| `check` (`check_project`)                  | **No**                             |
| LSP in-project (`check_source_in_project`) | **No**                             |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see "Target Design" 3), otherwise this path will be missed when unifying
Driver.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — goes to `06-cleanup-inventory.md`. The open question in RFC-039 "build wasm
target, or delete these branches" is **already resolved**: the target has been built.)

---

## Target Design

### 1. Stage model: `Stage` enum (exhaustive, not runtime-registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can compile-time enforce "new stages must be handled by
the orchestration layer" — which is exactly the mechanism that the opcode table has already proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check         Project
    Discovery,          // File discovery                   Project
    Registry,           // Module registry construction     Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Parsing,            // Lexing + parsing                 PerModule
    Typecheck,          // Type check (including embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis        Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation           Project
    Monomorphization,   // Monomorphization                 Project
    IrGeneration,       // AST → ModuleIR                   PerModule
    Linking,            // Cross-module linking / IR merge  Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at project level) make "which stages must run per module and which must run once at project level" a
type-level fact. Project-scoped stages are structurally guaranteed to run only once in `dispatch` —
this eliminates problems in current `orchestrator.rs` like "should `allocate_global_slots` be called
once per file" that require human reasoning.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on order.

### 2. Obligations ledger: `Obligations` + `assert_drained()`

**Design goal**: make "field produced but no one consumes" detectable at compile time or test time
instead of undetectable. RFC-039 has already listed this as "the highest-value item."

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
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                              | Disposition                                                                         |
| ------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| Obligation read by declared consumer | The field in `Obligations` is `take()`/marked consumed | Pass                                                                                |
| Obligation non-empty but no consumer | Field non-empty and not consumed                       | **Produce diagnostic** (`W` level, default; in `strict` mode, upgrade to `E` level) |
| Obligation empty                     | Field is empty                                         | Pass (no need to consume)                                                           |

**Why start with W level rather than E level**: fixing obligation dropping **changes the diagnostic
set**. The C2 criterion requires "identical diagnostic sets across entries"; if we go straight to E
level, `yaoxiang check` will suddenly have a large number of previously silent `Unproven`
diagnostics. Two steps (first W observe, then upgrade to E) keep each step's criterion usable. See
"Implementation Notes" S4.

**The call site of `assert_drained()` is unique**: `Driver::run` at the end of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third-file read → fail). `assert_drained()` is a **runtime** gate, the script is a
**static** gate, the two are complementary and both are needed.

### 3. Unified Driver: single `dispatch`

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
    /// Can only come from one of 6 predefined combinations of `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be the 6 predefined combinations above (one per `ProgramKind`); the caller is
not allowed to freely pass arrays.** This is the key constraint preventing the `Program` abstraction
from degrading into "free parameter that can pass anything," asserted by
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
            match stage {                            // exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Only settlement point
        Ok(state.into_result())
    }
}
```

**Migration plan for the ten entries (function-by-function)**:

| Entry                                                      | After migration                                                                                                                                                                                                 |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                  |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                         |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                   |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                     |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter results to target file                                                                                                                                                       |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                       |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (forwards to `run_project` or `SingleFile`)                                                                                                                                                           |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (forwards to Driver)                                                                                                                                                                                  |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project, change to `Program { kind: Lsp, aggregation: CollectAll }`; **delete the manual lex→parse→… sequence in the single-file branch**, change to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete `standalone` branch (`614-616` → `check_single_file` `623-661`), uniformly use `Program { kind: Check }`                                                                                                 |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% diverged)**. The
shared part is: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, resolution loop, per-file checker assembly, result processing. The diverged
part is items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton is exactly
the value zone of `Driver`** — the 60% divergence is all "different stage set" or "different
aggregation mode", and that is exactly what the two fields of `Program` can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "skipped due to upstream failure" is not part of the stage return value.**

Reason: skipping is **topology**-determined, not stage-determined. If we let each stage return
`Skipped` itself, the information "why didn't I run" gets scattered across 12 stages and cannot be
centrally audited. Instead, the dispatch determines it at the start of the loop based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
aims to fix — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip" that
produces no diagnostic, just because the obligation happens to be empty. After the rule is
established, any "because X wasn't done, Y didn't run" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Wording direction                                                                                                                         |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` caused                                  | "Stage Y did not execute because upstream stage X failed"                                                                                 |
| Pre-condition obligation empty caused                    | "Stage Y did not execute because there are no pending obligations (normal)" — **this item severity = Info, not counted in warning count** |
| Condition not met (e.g., `config.mono.enabled == false`) | "Stage Y did not execute because configuration is not enabled"                                                                            |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of this count).

### 5. Diagnostic aggregation mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                                  | Current correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error return `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)                     | Already exists         |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` does not exit early), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists         |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project and single-file within the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the semantic difference of the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of "early
exit or not" — they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter, not retain two functions (**this is an additional
verification needed for this document, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type system impact

| Change                                      | Type layer impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Does not introduce new type representations, does not touch `MonoType` / `PolyType` / `ir::Type`                                           |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what is migrated into the ledger is **settlement
responsibility**, not type ownership. `ReleasePlan` is still defined in `layers/ownership.rs`,
`ProofFunctionCall` is still in `proof/`, and other field types stay where they are;
`driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g., `ir_gen` reads `release_plan`) continue to receive specific field types via parameters,
**must not `use crate::driver`** — this is consistent with the no-reverse-dependency red line on
driver below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not
reverse-`use crate::driver`**. `Driver` appearing in `TypeChecker`'s imports is a violation,
intercepted by `scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime behavior

| Scenario                            | Before                                   | After                                                                                     |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                             |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | **Add** proof_execution; **add** warning output                                           |
| `yaoxiang check` (multi-file)       | No proof_execution                       | **Add** proof_execution                                                                   |
| LSP (in-project)                    | No proof_execution                       | **Add** proof_execution                                                                   |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**) |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                    |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract already declared by `backend.rs:60` ("caller should conservatively degrade") — the
current implementation is the opposite of its own contract document.

### Compiler change list

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file, N for multi-file)                          |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line range                         | Change                                                                                                                                                         |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                          |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                 |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                               |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                               |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                         |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and serves as the implementation of `Stage::ProofExecution`                                                |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed down to Program constructor                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed down to Program constructor                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to use Driver                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s choice between `check_module` / `check_module_collect_all` now driven by `Aggregation` parameter                                |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`' manual stage sequence deleted, changed to call Driver                                                                                       |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` uniformly uses `Program { kind: Check }`                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`) |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent drop point**)                                                |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix comment inconsistent with `1320`                                                                                                                           |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "lower-layer failure prevents upper-layer from running" (no short-circuit exists)              |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | `default_solver()` changed to return `&'static` singleton reference (fix per-back-edge new context)                                                            |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logic is itself
correct — **the problem is at the consumer, not the producer**, changing the producer would mask the
architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch (reachability of
wasm branches goes to `06-cleanup-inventory.md`).

### Backward compatibility

| Change                                                    | Compatibility                                                            | Disposition                                                                                                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaks**: previously silently passing constraints now report E4018     | `test_multifile_proof_obligation_not_dropped` **write red first then fix**; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaks**: previously compiling-clean programs start producing warnings | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code unchanged                                  |
| `assert_drained()` first run produces W-level diagnostics | **Breaks**: diagnostic set adds                                          | First W then E, S2/S4 two steps                                                                                                                      |
| `predicate.rs` panic changed to diagnostic                | **Improves**: no longer crashes                                          | No break                                                                                                                                             |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                            | These two paths don't go through proof_execution                                                                                                     |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactor**                                                    | If `pub` API surface has external dependencies, needs sync; in-repo consumers are fully listed in the change list                                    |

---

## Implementation Notes

This document corresponds to RFC-039 global stage sequence's **P3 (fix correctness
vulnerabilities)** and **P4 (stage contract and unified Driver)**. The following S1-S5 are the
internal implementation order of this document, **each step's acceptance criteria references the C2
category of the [Equivalence Oracle document](07-equivalence-oracle.md)** (orchestration changes:
identical **diagnostic sets** across entries + identical corpus behavior).

### S1: Establish criteria (red first)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written as failing first.

| Deliverable                                                                                                                                               | Acceptance                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                             | **Must be red**. If accidentally green, this document's vulnerability analysis needs re-review |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48 — not mixed into single-file corpus tree; project fixture with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file two versions of the corpus can be compared       |
| `test_obligations_drained` skeleton                                                                                                                       | Marked `#[ignore]`, turns green at S4                                                          |

**Rollback point**: no code changes, purely new tests.

### S2: Introduce `Stage` + `Program`, no behavior change

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code are byte-for-byte identical**                |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately does not fix any bug** — it just moves existing behavior into Driver.
Acceptance criteria is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restored to original is enough.

### S3: Merge orchestrator's four entries + fix vulnerability

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Diagnostic sets of four entries identical after normalization by `Aggregation`    |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in and out of project                       |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give identical diagnostic sets for the same file                      |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics without Z3 environment                                           |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If
structural merge has issues, only the structural commit can be rolled back while keeping the
vulnerability fix commit — at which point `test_multifile_proof_obligation_not_dropped` stays green;
the reverse (keeping structure, rolling back fix) will turn red, which is an unacceptable
intermediate state, prohibited from merging.

### S4: Enable obligations ledger

| Deliverable                                  | Acceptance                                                 |
| -------------------------------------------- | ---------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                     |
| `scripts/ci/check-obligations.py`            | Static gate effective                                      |
| Obligation diagnostic W → E upgrade          | Manual review of each newly-added E after full corpus diff |

**Rollback point**: The severity of `assert_drained()` can be controlled by config, W/E switch
doesn't require code structure change.

### S5: Proof layer and wasm wrap-up

| Deliverable                                               | Acceptance                                                                                      |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order    | Document corresponds item-by-item with `checker.rs` call sites                                  |
| `backend.rs:67-72` changed to singleton                   | Cache hit rate observable (add counter)                                                         |
| `checker.rs:1283-1293` degradation adds warning           | Diagnostic when no `body_checker`                                                               |
| `orchestrator.rs` 12 wasm attribute reachability judgment | Conclusion goes to `06-cleanup-inventory.md`, this document only registers judgment requirement |

**Note**: fixing layer order will change the diagnostic set, may expose large amounts of previously
silent `Unproven`. **This item should be done independently of S1-S4**, not mixed in the same PR as
entry merging.

## Key Decisions and Rationale

| Decision                 | Decision                                                                                                                                                | Rationale                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **runtime registration rejected**                                                                 | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant, `dispatch` fails to compile. This is isomorphic to the mechanism already validated for the opcode table in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating call-count problems that need human reasoning               |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, paired with `scripts/ci/check-obligations.py`'s **static** gate; severity first W then E | What's being fixed is a whole class of bugs, not one bug: currently verified same-class vulnerabilities at least 2 (`proof_calls`, `checker.rs:1313`), all 16 span-keyed fields are in range. First W then E is to keep each step's C2 criterion usable — going straight to E will make `yaoxiang check` suddenly have a large amount of previously silent diagnostics |
| **Fix scope**            | Fix the **consumer** (orchestration layer), **don't touch** the six `Unproven` branch production logic at `checker.rs:5164/5179/5306/5318/5420/5448`    | The problem is at the consumer, not the producer. Changing the producer would mask the architectural defect as "logic fixed", while these six branches' logic itself is correct                                                                                                                                                                                        |

This design additionally eliminates two classes of cracks:

- **Crack between "comment promises" and "code behavior"**. The "must not let silent pass come back"
  at `checker.rs:5165` and "lower-layer failure prevents upper-layer from running" at
  `layers/README.md:3` are both **comment-level contracts** — the former is fulfilled by
  `assert_drained()`, the latter is exposed by topology-driven `Skipped` diagnostics.
- **"Forgot to append to an array"**. Currently five functions each manually write their own call
  sequence; after unification it's the only one stage table, and missing wiring of a new stage will
  fail at compile time.

### Rejected directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — of the 11 inconsistencies, 7
  (dead code analysis two implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type interface**, but are "whether to run a certain analysis"
  configuration issues; it cannot express `Aggregation` either, and can only move the choice between
  `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when adding new stages, `dispatch` no longer fails to compile, equivalent to
  replacing "five functions each manually write their own calls" with "forgot to append to an
  array", which is exactly the cause of the 11 inconsistencies. Same-form counterexample already in
  this repository: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implementation.
- **Only patch bug, don't touch structure** — can fix the 2 known bugs, but only covers 1 of 16
  obligation fields, `Stage` is still scattered across 5 functions, the next divergence point will
  continue to grow from here. **It must be done first** (it's part of S1/S3), because structural
  change needs a known red test to prove the criterion is effective.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **wholly** discarded. This is looking for a bug at the wrong level:
  the bug is at the orchestration layer, detection must be at the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to span
  two files to understand the flow. This is a common cost of all "centralize the wiring" refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silent problems"**. After the fix, there may be a batch of user reports of "new errors"
  — they are real bugs that just never got reported before. Must be made clear in CHANGELOG.
- **`assert_drained()`'s severity switch needs per-field manual judgment**. Of the 16 fields, some
  (e.g., `module_namespaces`) may be designed to not need consumption, and should not warn. S4 needs
  to go through each field, not blanket.
- **`Program` abstraction may be premature**. If some entries' stage sets are unstable in the long
  term, `stages()` will degrade into "pass different array each call" free parameter, and the
  contract constraint fails. **Mitigation is `test_program_stage_coverage` asserting that `stages()`
  can only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criteria depend on the equivalence oracle
  document's vulnerability test going red first. **If that document doesn't establish a multi-file
  corpus layer first, S1 cannot be accepted** — because the existing 293 corpus files all go through
  the single-file path, with zero coverage for this type of defect. This document doesn't cover the
  IR verifier (`verify_loose`), that prerequisite work is outside this document's scope.
- **Layer order fix will expose previously silent `Unproven`** (`equivalence` is not in the pipeline
  at all, `termination` and `ownership` order is opposite from the declaration). This is a
  diagnostic set change risk, should be done independently.
- **Thread safety of SMT backend after changing to singleton is undecided**. `backend.rs:13-19`
  already states `Solver` is `Send` but not `Sync`, `Z3Backend`'s cache is `RefCell`; after changing
  to a singleton shared across compilation units, need to confirm access paths all go through
  `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (like the actual order dependency between
  termination and ownership). Should be opened after the equivalence criterion is stable.

> **The open questions originally listed in this section are all decided**. Per-item decisions are
> in [RFC-039 Decision Log](../../rfc/draft/039-compiler-architecture.md) (D1–D50). **This document
> has no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup; `include!`
  migration of `checker/semantic_tokens.rs` (construction steps go to [09](09-execution-wbs.md) §P5
  5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit statement of "must not let silent
  pass come back"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit statement of "lower-layer failure
  prevents upper-layer from running" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit rationale for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract statement of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test's
  self-acknowledgment of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus's
  self-acknowledgment of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path,
  therefore executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate rather than main crate
- `build.rs:19-55` — Build-time gate for error codes, example of this project's enforcement
  mechanism
