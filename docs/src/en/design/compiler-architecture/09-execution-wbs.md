# Multi-level Work Breakdown Structure (WBS)

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md), expanding
> the RFC's phase sequence into **independently committable, independently verifiable** three-level
> tasks.
>
> Phase ordering and acceptance gates are governed solely by RFC-039; this document is only
> responsible for breaking things down to executable granularity and registering unresolved
> conflicts.

## Positioning and Scope

| Level       | Count       | Meaning                                                        |
| ----------- | ----------- | -------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Phases, corresponding one-to-one with RFC-039's phase sequence |
| **Level 2** | 45          | Task groups, from each document's "Implementation Points"      |
| **Level 3** | 122         | Independently committable actions                              |

**This document covers**: task breakdown, dependencies, parallel grouping, binding to acceptance
criteria.

**This document does not cover**: design rationale for tasks (see `02`–`08`), justification of phase
ordering (see RFC-039).

## Current State: Gaps Found During Breakdown

When consolidating the "Implementation Points" from the 8 documents into this table, **5 hard gaps
in the document system itself were found**. All have been converged:

| #   | Gap                                                                                                                                                                                                                                                                                                                                     | Convergence Result                                                                                                                                             |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no implementation steps**. The RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents describe how to do this split; the directory tree in `01-routing.md` mentioned `annotations.rs` once — the only mention in the entire batch                                                           | P5 steps are filled in by this document (see below)                                                                                                            |
| G2  | **`include!` refactor mentioned in three places, landed in none**. Listed under P4 in the RFC; `02`'s cross-reference points to itself; `06`'s S7 also lists it. But `02`'s change list (6 additions + 20 modifications) does **not include `semantic_tokens.rs`**                                                                      | Implementation steps assigned to this document §P5 5.1 (after P4 completes); references in RFC and 06 now point here                                           |
| G3  | **Three parallel lists of CI scripts**. The RFC lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists 4 other checks (all differently named); `04` also wants `check-synth-boundary.py`. After deduplication **at least 9 new scripts, no unified list**; `check-obligations.py` is doubly assigned to P4 and P9 | Unified list in this document §P9 (10 scripts, each assigned to one phase); other documents all reference that table                                           |
| G4  | **`precedence_inline.rs` life-or-death conflict**. `06` counts it among "1005 lines that need resurrection", but the same document's B6 says it must be deleted together with the `Precedence` enum, and `05` advocates deletion                                                                                                        | **P1 resurrects, P8 deletes with Pratt** (decision registered as C1) — first restore CI coverage, the deletion happens in P8's 8.6 along with the dead ladders |
| G5  | **L2 target statement conflict**. The RFC and this directory's index say "grammar-driven syntax", but `05`'s self-assessment says "**did not use true grammar-driven (LALRPOP), only achieved declarative table lookup**"                                                                                                               | Converged to **full LALRPOP grammar-driven** (`05` §Syntax: Complete Grammar-Driven); G7/G8 acceptance gates are executed per this commitment                  |

## Target Design: Phase Sequence

```
P0  Repository maintenance mechanism and code placement regulations  08        ← new, rules first
P1  Resurrect orphan tests and fix defects                           06 §S1
P2  Establish equivalence oracle baseline                            07 all
P3  Fix correctness vulnerabilities (minimal approach)               02 §S3 - vulnerability half
P4  Stage contract and unified Driver                                02
P5  checker file-internal split                                      02 + supplemented by this doc
P6  Unify type representation                                        03
P7  SSA-ify intermediate representation                              04      } parallelizable
P8  Frontend paradigm change                                         05      }
P9  Anti-regression gates (with unified script list)                 01 §Anti-regression + 08 §Machine-checkable rules
P10 Other cleanup and status corrections                             06 §S2/S3/S4/S5/S6
```

**Rationale for the new P0**: The three prohibitions (no fabrication / no infinite padding /
refactoring needed but only patched) constrain every action in P1–P10. Rules must be established
before construction begins, otherwise P1's test resurrection will already be adding `mod`
declarations to `tests/mod.rs` — which is exactly what caused the G4 conflict.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Regulations

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [ ] **0.1 Regulations written** (3 level-3 tasks; touch points of 0.1.2 and rules extraction of
      0.1.3 were landed early on 2026-10-05, leaving 0.1.1 CONTRIBUTING body review)
- [ ] **0.2 Gate implementation** (3 level-3 tasks)
- [ ] **0.3 Table gate promotion** (1 level-3 task)

| Level 2                  | Level 3                                                                                                                                                                                                                                                                                                                                                    | Pre-req | Acceptance                                                                                                                                                                       |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Regulations written  | 0.1.1 Write D0–D4 + three prohibitions into `CONTRIBUTING.md`; responsibility categories reference `01-routing.md`'s directory responsibility table                                                                                                                                                                                                        | —       | Human review passes                                                                                                                                                              |
|                          | 0.1.2 Implementer touch points landed (**completed 2026-10-05**): create `AGENTS.md` (root entry) + `compiler-architecture/HOWTO.md` (pre-work self-check + D3 patch determination); add "Responsibility Attribution and Decision Procedure" required block to PR template (D44); add "Code Placement and Change Regulations" section to `CONTRIBUTING.md` | —       | Landed; review verifies content of four touch points                                                                                                                             |
|                          | 0.1.3 Rule body extraction (**completed 2026-10-05**): create `docs/src/dev/coding-rules.md` (precise rule descriptions, long-term valid, not archived with RFC-039); add migration header to 08 converting it to a diagnostic record; all four touch points now reference coding-rules                                                                    | —       | Landed                                                                                                                                                                           |
| 0.2 Gate implementation  | 0.2.1 `check-concepts.py` (prohibition 1 A/B/C/D)                                                                                                                                                                                                                                                                                                          | 0.1.1   | **Must report 3 sets of operator enums and 2 sets of parallel type representations (including `ir::Type` aliases) on unmodified code**; report-only in this phase, hardens in P9 |
|                          | 0.2.2 `check-fanout.py` (prohibition 3 A/B/C)                                                                                                                                                                                                                                                                                                              | 0.1.1   | **Deliberately adding a 6th entry-style wiring must turn red**; report-only in this phase, hardens in P9                                                                         |
|                          | 0.2.3 `check-boundary.py` (prohibition 2 criterion C)                                                                                                                                                                                                                                                                                                      | 0.1.1   | **Deliberately adding one L2→L3 reverse `use` must turn red**; `pub(crate)` leakage count records initial value                                                                  |
| 0.3 Table gate promotion | 0.3.1 Extend `tools/code-tables` to cover opcode / type tables                                                                                                                                                                                                                                                                                             | 0.1.1   | Depends on P6's T1; closed separately                                                                                                                                            |

> **No scale-baseline task**. The 2026-10-03 decision cancels all line-count / volume gates; scale
> issues are resolved by responsibility separation (see `08` prohibition 2 criterion A, human
> judgment). **0.2.1 is the core acceptance for this phase**: if the gate does not report known
> parallel representations, the criterion design is wrong — it's a fake gate.

### P1 Resurrect Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Points S1 + [05](05-frontend-paradigm.md)
§Test Reconstruction

- [ ] **1.1 Resurrect `lexer/tests/`** (4 level-3 tasks)
- [ ] **1.2 Resurrect 4 places in live directories** (1 level-3 task)
- [ ] **1.3 Fix exposed defects** (2 level-3 tasks)

| Level 2                                    | Level 3                                                                                                                                                 | File:Line                  | Pre-req | Acceptance                                                        |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------- | ----------------------------------------------------------------- |
| 1.1 Resurrect `lexer/tests/`               | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                     | after `lexer/mod.rs:106`   | —       | Test count rises                                                  |
|                                            | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                             | `lexer/tests/mod.rs:15-25` | 1.1.1   | **159 lines still don't run without this**                        |
|                                            | 1.1.3 Delete 7 empty shell files + their `mod` declarations                                                                                             | 7 files in `lexer/tests/`  | 1.1.1   | No C6 criterion                                                   |
|                                            | 1.1.4 Delete `pub use ...::*;` re-export block                                                                                                          | `tests/mod.rs:28-38`       | 1.1.3   | This block is what hides the empty shells                         |
| 1.2 Resurrect 4 places in live directories | 1.2.1–1.2.4 respectively wire `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` | each parent module         | —       | Test count rises; **1.2.1 conflicts with 8.0.1, see conflict C1** |
| 1.3 Fix exposed defects                    | 1.3.1 Overflow paths in 4 radix scanners                                                                                                                | `literals.rs`              | 1.1.x   | `cargo test` green                                                |
|                                            | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                    | `literals.rs`              | 1.1.x   | Same as above                                                     |

**Acceptance**: test count rises; **exposing real defects is expected**, so reserve time for fixes.
Revert file by file, but **the fixed defects should not be rolled back**.

### P2 Establish Equivalence Oracle Baseline

Source: [07](07-equivalence-oracle.md) all (this document has no "Implementation Points" section,
reversed from structure)

- [ ] **2.1 IR static verifier** (3 level-3 tasks)
- [ ] **2.2 Normalized snapshots** (2 level-3 tasks)
- [ ] **2.3 Corpus diff** (3 level-3 tasks)
- [ ] **2.4 Vulnerability-specific oracles** (3 level-3 tasks)
- [ ] **2.5 Regression gate** (1 level-3 task)

| Level 2                            | Level 3                                                                                                                                                   | Pre-req | Acceptance                                                                              |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | --------------------------------------------------------------------------------------- |
| 2.1 IR static verifier             | 2.1.1 `verify.rs` dual modes `verify_loose` / `verify_ssa`                                                                                                | —       | C4                                                                                      |
|                                    | 2.1.2 **`verify_loose` runs green on existing non-SSA IR**                                                                                                | 2.1.1   | **Hard threshold, entry condition for P7 batch a**                                      |
|                                    | 2.1.3 Implement 7 invariants (dominance / unique definition / Phi consistency / jump targets / global out-of-bounds / type consistency / inner isolation) | 2.1.1   | C4                                                                                      |
| 2.2 Normalized snapshots           | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / order predecessors)                                                        | —       | C1/C3/C5                                                                                |
|                                    | 2.2.2 Snapshot check-in + human review process (`src/middle/core/tests/snapshots/`)                                                                       | 2.2.1   | C1/C3/C5                                                                                |
| 2.3 Corpus diff                    | 2.3.1 Diff framework + 293 corpus baseline                                                                                                                | —       | Diagnostics/exit code/stdout item by item                                               |
|                                    | 2.3.2 **Multi-file corpus layer** (create `tests/yaoxiang-multifile/`, with project fixtures containing `yaoxiang.toml` — decision D48)                   | —       | **Unconditionally required (D40), the only executable behavioral oracle source for P4** |
|                                    | 2.3.3 C4 behavioral diff must-cover list (10 semantic categories) + performance baseline (criterion smoke benchmark)                                      | —       | C2/C4/C5; performance baseline for P4/P7/P8 comparison                                  |
| 2.4 Vulnerability-specific oracles | 2.4.1 `test_multifile_proof_obligation_not_dropped`                                                                                                       | 2.3.2   | **Must be red first**                                                                   |
|                                    | 2.4.3 `test_no_silent_pass_on_unproven`                                                                                                                   | —       | Catches `checker.rs:5179/5318/5448`                                                     |
|                                    | 2.4.4 `test_release_plan_spans_consumed`                                                                                                                  | —       | **No whitelist, difference set must be empty (D41)**                                    |
| 2.5 Regression gate                | 2.5.1–2.5.4 see P9 unified list                                                                                                                           | 2.1–2.3 | See P9                                                                                  |

> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **moved out of P2**: the `Program` / `Obligations` types they reference only exist in P4, so P2
> cannot compile. The former is P4's 4.1.4 (no double registration), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.

### P3 Fix Correctness Vulnerabilities (Minimal Approach)

Source: [02](02-stage-contract.md) §Implementation Points S3's vulnerability-fix half

- [ ] **3.1 `proof_calls` consumer side** (2 level-3 tasks)
- [ ] **3.2 Second silent-drop point** (1 level-3 task)
- [ ] **3.3 panic to diagnostic** (1 level-3 task)

| Level 2                         | Level 3                                               | File:Line                                | Pre-req | Acceptance                          |
| ------------------------------- | ----------------------------------------------------- | ---------------------------------------- | ------- | ----------------------------------- |
| 3.1 `proof_calls` consumer side | 3.1.1 Make `proof_calls` private + sole getter        | `types.rs:29`                            | 2.4.1   | Vulnerability oracle turns green    |
|                                 | 3.1.2 Fill in the three consumer points               | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1   | C2                                  |
| 3.2 Second silent-drop point    | 3.2.1 `Unproven` empty match arm produces diagnostic  | `checker.rs:1303-1314`                   | 2.4.3   | `test_no_silent_pass_on_unproven`   |
| 3.3 panic to diagnostic         | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic | `predicate.rs:34-36`                     | —       | No longer panics when Z3 is missing |

**Acceptance**: vulnerability oracles turn green, and **deliberately reverting the fix must turn red
again**. **Risk**: adding E4018 to the multi-file path is a breaking change.

> **Bleeding channel (2026-10-05 ruling)**: P3 does not need to wait for P2 to complete. The only
> hard prerequisites for 3.1/3.2 are **2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red
> oracles)**; 2.1/2.2/2.3.1/2.3.3 (IR verifier, snapshots, single-file diff, performance baseline)
> can proceed in parallel with P3. The bleeding of correctness vulnerabilities (`Sorted(3)` silent
> pass) should not be blocked by snapshot infrastructure construction. 3.3 (panic to diagnostic) has
> no prerequisites and can land at any time.

### P4 Stage Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Points S1–S5. **The fixes in this phase and P3
must be split into two commits.**

- [ ] **4.1 Declarative stage table** (5 level-3 tasks)
- [ ] **4.2 Entry merge** (8 level-3 tasks)
- [ ] **4.3 Obligations ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm wrap-up** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ization (D20)** (2 level-3 tasks)

| Level 2                                       | Level 3                                                                                                                              | Pre-req | Acceptance                                                                                                                          |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative stage table                   | 4.1.1 `stage.rs` (`Stage` variants + `Stage::ALL` + `StageScope`)                                                                    | P0      | `test_program_stage_coverage`                                                                                                       |
|                                               | 4.1.2 `program.rs` / `unit.rs`                                                                                                       | 4.1.1   | Same as above                                                                                                                       |
|                                               | 4.1.3 `Pipeline::run` routed through Driver (`pipeline.rs:141-227`)                                                                  | 4.1.2   | **Single-file diagnostic set and exit code byte-identical**                                                                         |
|                                               | 4.1.4 Stage coverage assertion                                                                                                       | 4.1.3   | C2                                                                                                                                  |
|                                               | 4.1.5 Full corpus zero-diff                                                                                                          | 4.1.4   | Diagnostics/exit code/stdout all zero-diff (293 corpus)                                                                             |
| 4.2 Entry merge                               | 4.2.1 Slim down `compile_project` to Program constructor (`orchestrator.rs:99-237`)                                                  | 4.1.5   | Diagnostic sets across four entries identical after unification                                                                     |
|                                               | 4.2.2 `check_project` (`:273-399`)                                                                                                   | 4.2.1   | Same as above                                                                                                                       |
|                                               | 4.2.3 `check_source_in_project` (`:450`)                                                                                             | 4.2.2   | Same as above                                                                                                                       |
|                                               | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                            | 4.2.3   | Same as above                                                                                                                       |
|                                               | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                       | 4.2.1   | `check` consistent inside and outside project                                                                                       |
|                                               | 4.2.6 Delete LSP manual stage sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                       | 4.2.1   | LSP and CLI diagnostic sets identical                                                                                               |
|                                               | 4.2.7 `Aggregation` parameter-driven choice (`orchestrator.rs:486-494`)                                                              | 4.2.1   | **Verify line by line the internal differences between the two functions during implementation** (conflict C5 ruled as must-verify) |
|                                               | 4.2.8 wasm routed through `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                              | 4.2.1   | See C6                                                                                                                              |
| 4.3 Obligations ledger                        | 4.3.1 `obligations.rs` + `assert_drained()` (new; including `#[ignore]` red skeleton first)                                          | 4.1.3   | `test_obligations_drained` turns green                                                                                              |
|                                               | 4.3.2 Obligation diagnostic W→E upgrade                                                                                              | 4.3.1   | Human review each new E                                                                                                             |
| 4.4 Proof layer and wasm wrap-up              | 4.4.1 `layers/README.md` layer order changed to actual order                                                                         | —       | **Independent of 4.1–4.3**                                                                                                          |
|                                               | 4.4.2 `default_solver()` changed to `&'static` singleton (`proof/smt/backend.rs:67-72`)                                              | —       | Cache hit rate observable                                                                                                           |
|                                               | 4.4.3 Add warning to degradation path (`checker.rs:1283-1293`)                                                                       | —       | —                                                                                                                                   |
| 4.5 Cross-layer contract PlanId-ization (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (`layers/ownership.rs:31` producer-side allocation + `ir_gen.rs:1942` consumer-side match) | 4.3.1   | `test_release_plan_spans_consumed` difference set empty (D41)                                                                       |
|                                               | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                   | 4.5.1   | C2; span mismatch-type silent failure goes to zero                                                                                  |

### P5 checker File-Internal Split — Supplemented by This Document (Original Gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (`collect_used_in_type`'s
`pub(crate)` path must not change)

- [ ] **5.1 `include!` → real `mod`** (2 level-3 tasks)
- [ ] **5.2 Split out `refinement`** (2 level-3 tasks)
- [ ] **5.3 Split out `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                                        | Pre-req     | Acceptance                                                   |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------ |
| 5.1 `include!` → real `mod` | 5.1.1 Change `include!` at `checker.rs:5618` to a real `mod` declaration                                                                                                                                                                                       | P4 complete | C1 zero-diff; `semantic_tokens.rs` gains module identity     |
|                             | 5.1.2 Add `use super::*` or item-by-item import (`include!` era implicitly inherited scope)                                                                                                                                                                    | 5.1.1       | `cargo build` green                                          |
| 5.2 Split out `refinement`  | 5.2.1 Migrate refinement block (current `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                                   | 5.1.1       | C1 zero-diff                                                 |
|                             | 5.2.2 `collect_refined_binding_checks` signature change: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                           | 5.2.1       | C1; sole call site is `1235`                                 |
| 5.3 Split out `annotations` | 5.3.1 Migrate annotation checks (current `3441-3661`, including `is_predicate_head`)                                                                                                                                                                           | 5.2.2       | C1 zero-diff                                                 |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) keeps `pub(crate)` and stays under the `checker` module path** → add `pub(crate) use annotations::*;` to the top of `checker.rs`, so that the import in `inference/statements.rs:16` **stays unchanged** | 5.3.1       | **Zero caller changes** is the hard acceptance for this step |

**Hard constraint**: `collect_used_in_type` is used cross-module by `inference/statements.rs:16`.
The approach is to use `pub(crate) use` re-export, making the split a **pure relocation**.

**Only split confirmed large blocks.** Do not split modules of only 200–400 lines — that increases
navigation cost, reduces cohesion, with zero decoupling benefit. The other sub-modules (signatures /
type_defs / imports) wait until the file grows back to 1,500 lines.

### P6 Unify Type Representation

Source: [03](03-type-unification.md) §Implementation Points Phases 0–6, criterion C3. **Phases 0→6
strictly serial.**

- [ ] **6.0 Gate first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead variant deletion** (2 level-3 tasks)
- [ ] **6.4 parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variants** (2 level-3 tasks)
- [ ] **6.6 Gate hardens** (1 level-3 task)
- [ ] **6.7 Directory rename (D1, pure relocation batch)** (1 level-3 task)

| Level 2                                          | Level 3                                                                                                                                                                                                 | File:Line                                                                 | Acceptance                                                                                                      |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| 6.0 Gate first                                   | 6.0.1 Create new `tools/type-tables` crate, implement T1/T2/T3, **T1 report-only initially**                                                                                                            | new crate                                                                 | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge rebuild points listed separately** |
|                                                  | 6.0.2 Take full baseline on unmodified code                                                                                                                                                             | —                                                                         | No code changes                                                                                                 |
| 6.1 Name normalization                           | 6.1.1 Count occurrences of non-canonical type names in corpus                                                                                                                                           | `tests/yaoxiang/`                                                         | Decides whether synonyms can be deleted directly                                                                |
|                                                  | 6.1.2 `from_builtin_name` cuts synonyms → lexer alias table                                                                                                                                             | `mono.rs:618-643`                                                         | C3                                                                                                              |
|                                                  | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                    | `types.rs:162-165`                                                        | **Must verify line by line, cannot assume equivalence**                                                         |
| 6.2 Bytecode type unification                    | 6.2.1 Change three places to `MonoType`                                                                                                                                                                 | `bytecode.rs:812/814/854`, `image.rs:44`                                  | C3 + `dump_bytecode` targeted comparison                                                                        |
|                                                  | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                | `bytecode.rs:2352-2390`                                                   | Same as above                                                                                                   |
|                                                  | 6.2.3 `type_table` element type changed to `MonoType`                                                                                                                                                   | `image.rs:44`                                                             | Same as above                                                                                                   |
| 6.3 Dead variant deletion                        | 6.3.1 Delete 11 variants + clear match arms + fallback                                                                                                                                                  | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                               | C3 + `cargo build` green                                                                                        |
|                                                  | 6.3.2 **Must be in the same batch as 6.3.1** rewrite the reverse bridge                                                                                                                                 | `passes/mono/function.rs:507-529` (`mono_to_ast_type` rebuilds 7 of them) | Generic replacement targeted comparison, otherwise **silently changes behavior**                                |
| 6.4 parser data flow                             | 6.4.1 Add new `probe.rs`                                                                                                                                                                                | new                                                                       | C3                                                                                                              |
|                                                  | 6.4.2 Change `parser_state.rs:46-58`                                                                                                                                                                    | —                                                                         | Same as above                                                                                                   |
|                                                  | 6.4.3 Delete `operator_interfaces::spec()` calls (**2 places**: `declarations.rs:509`, `ast.rs:930`)                                                                                                    | —                                                                         | Same as above                                                                                                   |
|                                                  | 6.4.4 Merge two `name_used_as_type*`                                                                                                                                                                    | `ast.rs:847` + `declarations.rs:42`                                       | Same as above                                                                                                   |
|                                                  | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) wired through `NameKind::Builtin`                                                                                                                              | —                                                                         | Same as above                                                                                                   |
| 6.5 AST dead variants                            | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                              | `ast.rs:37-43` + 4 consumer sites + 2 exhaustive arms                     | `tests/integration/` 18 modules green (**split into 2 commits**)                                                |
|                                                  | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                         | `ast.rs:241-249`                                                          | Mark **semantic risk**                                                                                          |
| 6.6 Gate hardens                                 | 6.6.1 T1 changes from warning to `panic!`                                                                                                                                                               | `tools/type-tables`                                                       | **Deliberately introducing a zero-construction variant must turn red**                                          |
| 6.7 Directory rename (D1, pure relocation batch) | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; `parser/ast.rs` split into top-level `ast/`; bytecode domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` moved to `middle/bytecode/` | entire repo                                                               | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline does not regress                      |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 SSA-ify Intermediate Representation

Source: [04](04-ssa.md) §Implementation Points batches a–d, criterion C4, **strictly serial within a
batch**.

- [ ] **7a Cut off multiple definitions** (3 level-3 tasks)
- [ ] **7b SSA form switch** (6 level-3 tasks; 7b.1 split into 1a/1b per DoD 6)
- [ ] **7c Implicit contract explicitization** (3 level-3 tasks)
- [ ] **7d Last** (2 level-3 tasks)
- [ ] **7e `.42` data loss fix** (3 level-3 tasks; 2315/2341 absorbed from "independent issue",
      2026-10-05)

| Level 2                              | Level 3                                                                                                                                       | Pre-req                             | Acceptance                                     |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | ---------------------------------------------- |
| 7a Cut off multiple definitions      | 7a.1 Delete 3 statement-level recycles                                                                                                        | **2.1.2 `verify_loose` runs green** | C4                                             |
|                                      | 7a.2 Replace 6 save/restore sites with RAII guard                                                                                             | 7a.1                                | C1 snapshot zero-diff                          |
|                                      | 7a.3 Unify calibration (`1798` vs `2001`)                                                                                                     | 7a.2                                | C1                                             |
| 7b SSA form switch                   | 7b.1a `ir.rs` structural change #1-2 (`Operand` variant convergence + `Instruction::Phi` introduction)                                        | 7a.3 + **P6 complete**              | C4; exclusive commit                           |
|                                      | 7b.1b `ir.rs` structural change #3-5 (`FunctionIR` / `BasicBlock` / type annotation fields)                                                   | 7b.1a                               | C4; exclusive commit                           |
|                                      | 7b.2 `next_temp_reg` changed to `Operand::Value`                                                                                              | 7b.1                                | Same as above                                  |
|                                      | 7b.3 `translator.rs` add `Phi` arm                                                                                                            | 7b.1                                | C4 + `.42` round-trip test                     |
|                                      | 7b.4 Linear scan allocator (within u8/255 slot model)                                                                                         | 7b.3                                | Same as above (+500~1000 lines)                |
| 7c Implicit contract explicitization | 7c.1 `synth.rs` boundary + boundary check script                                                                                              | 7b.4                                | C4 + new script added to CI                    |
|                                      | 7c.2 span consumption count                                                                                                                   | 7c.1                                | Consumption count **zero triggers**            |
|                                      | 7c.3 `method_def_ordinals` made read-only                                                                                                     | 7c.1                                | Attribution questionable (C7)                  |
| 7d Last                              | 7d.1 Split `generate_call_expr_ir` into `CallArgs` + 6 `emit_*`                                                                               | 7c.x                                | C1 snapshot zero-diff                          |
|                                      | 7d.2 "Fill 0" fallback changed to return diagnostic                                                                                           | 7d.1                                | C4                                             |
| 7e `.42` data loss fix               | 7e.1 `upvalue_count: 0` fix (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + recycle already-allocated unused opcodes (D32/D34 with version bump) | 7d.x                                | C4 + `.42` round-trip test; exclusive commit   |
|                                      | 7e.2 `exception_handlers` persisted (`bytecode.rs:2315` — throw/try are core language semantics, **must not be left unimplemented**)          | 7e.1                                | `.42` direct run exception use case diff       |
|                                      | 7e.3 `globals` persisted (`bytecode.rs:2341`)                                                                                                 | 7e.1                                | `.42` direct run global variable use case diff |

**Expectation management**: net code **grows by 900–1600 lines**. If "code becomes shorter" is the
success criterion at project setup, this phase will be judged as failed.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Points Phases 0–4, criterion C5.

- [ ] **8.0 Dead ladder cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexical convergence** (2 level-3 tasks)
- [ ] **8.3 Build LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual-parser diff** (1 level-3 task)
- [ ] **8.5 Cutover** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 `parse_assign_after_target` split** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)

| Level 2                               | Level 3                                                                                           | Pre-req                          | Acceptance                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead ladder cleanup (C6)          | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                   | **Conflicts with 1.2.1, see C1** | No C6 criterion                                                                                                  |
|                                       | 8.0.2 Name bare magic numbers (`(6,7)` / `(11,1)` / `12`)                                         | —                                | Same as above                                                                                                    |
| 8.2 Lexical convergence               | 8.2.1 Merge 4 radix scanners + 3 escape merges + multi-line string merge                          | —                                | C5; `literals.rs` reduces by ~500 lines                                                                          |
|                                       | 8.2.2 Eliminate f-string nested compilation                                                       | 8.2.1                            | **spans necessarily change**, baseline must be set first                                                         |
| 8.3 Build LALRPOP grammar             | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                     | 8.2.x                            | `cargo build` passes; can produce AST for 293 corpus (**not compared**)                                          |
|                                       | 8.3.2 Action code `grammar/actions.rs` (one function per 22 `Expr` variant)                       | 8.3.1                            | Same as above                                                                                                    |
|                                       | 8.3.3 Error productions (preserve `Expr::Error` / `StmtKind::Error` placeholder semantics)        | 8.3.2                            | Same as above                                                                                                    |
| 8.4 **Dual-parser diff**              | 8.4.1 Run both parsers on 293 corpus + `src/std/tests`, normalize AST then **compare bit by bit** | 8.3.3                            | **Legal program AST fully equivalent — this is the equivalence proof for the entire grammar migration**          |
| 8.5 Cutover                           | 8.5.1 `parse()` calls LALRPOP, Pratt left as `parse_legacy()`                                     | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span identical item by item (**no relaxation**, no C5′ exists) |
| 8.6 Delete Pratt                      | 8.6.1 Delete `nud.rs`(1326) + `led.rs`(451) + two sets of BP ladders + ~89 references             | 8.5.1                            | Full corpus green; `git grep BP_` zero hits                                                                      |
|                                       | 8.6.2 Delete `is_old_function_syntax` (36 lines, natural invalidation)                            | 8.6.1                            | Same as above                                                                                                    |
| 8.7 `parse_assign_after_target` split | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                            | 8.6.x                            | This function only dispatches                                                                                    |
| 8.8 Test completion                   | 8.8.1 Add associativity cases (**red first then green**)                                          | 8.2.x                            | C5                                                                                                               |
|                                       | 8.8.2 Add 4 types of literal error paths                                                          | —                                | Cover untested paths                                                                                             |
|                                       | 8.8.3 Add `pratt/tests/mod.rs` wiring self-check assertion                                        | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6, changed to `parser/tests/`**)            |

**Risk**: 8.4 dual-parser diff is **an unavoidable equivalence checkpoint**. If it shows
non-equivalence, it must be resolved before 8.5 cutover, not hidden behind C5′ after cutover. If
8.6's ~89 BP reference migrations are missed, they **silently change associativity** (no error, only
parse result changes).

> **8.3–8.6 depend on `08`'s D0/D2**: if the grammar file is placed outside `parser/`, or action
> code reverse-references `sema`, it will be caught by `check-boundary.py`.

### P9 Anti-regression Gates (Unified Script List — Supplemented by This Document for Original Gap G3)

Source: [01](01-routing.md) §Anti-regression Mechanism + [08](08-maintenance-mechanism.md)
§Machine-Checkable Rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts are added to CI at
their "introduction phase"; report-only ones harden uniformly in P9 (change from "report" to "fail")
— existing violations (parallel representations, disambiguation aliases, etc.) won't be cleared
until P6/P8, and hardening too early will leave CI red long-term, with gates bypassed manually. By
P9 hardening time the gates should be exactly all green. P2's 2.5, P4's 4.3.2, P7's 7c.1 all
reference this table, no separate lists.

- [ ] **9.1–9.10 ten scripts added to CI per their introduction phase, harden uniformly in P9**
      (details in the table below)

| #    | Script                    | What it checks                                                                                                        | Intro Phase                                           | Acceptance                                                    |
| ---- | ------------------------- | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | Two-way assertion: `Program::stages()` × `Stage::ALL`                                                                 | P4                                                    | Deliberately omitting one stage must turn red                 |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third file reads it                                                | P4 (**sole attribution, no longer duplicated in P9**) | Deliberately leaving one field with no consumer must turn red |
| 9.3  | `check-boundary.py`       | Ban `include!`; `pub(crate)` cross-layer leakage only decreases; ban L2→L3 (**sole attribution**, defined by `08` P0) | **P0**                                                | Deliberately adding `include!` must turn red                  |
| 9.4  | `check-test-wiring.py`    | Has `tests/` directory but parent module lacks `mod tests;` → fail                                                    | P1                                                    | Deliberately creating an orphan directory must turn red       |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message lacks `snapshot-update` marker                                                   | P2                                                    | Same as above                                                 |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                  | P2                                                    | Same as above                                                 |
| 9.7  | `check-corpus-parity.py`  | Corpus diff non-empty (diagnostic code+span compared item by item, **no grouping relaxation**)                        | P2                                                    | Same as above                                                 |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction appearing outside `synth.rs`                                                                 | P7                                                    | Same as above                                                 |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representations, disambiguation aliases, synonym table)                               | **P0** (report-only, hardens in P9)                   | Must report 3 sets of operator enums on unmodified code       |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                   | **P0**                                                | Deliberately adding a 6th entry-style wiring must turn red    |

> **No line-count ratchet script** (`check-file-size.py` + `baseline.toml` cancelled on 2026-10-03).
> Scale issues are resolved by `08` prohibition 2 criterion A (responsibility separation, human
> judgment).

### P10 Other Cleanup and Status Corrections

Source: [06](06-cleanup-inventory.md) §Implementation Points

- [ ] **S2 Delete pure placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty design documents** (1 level-3 task)
- [ ] **S4 RFC status corrections** (1 level-3 task)
- [ ] **S5 Reduce `pub` visibility** (1 level-3 task)
- [ ] **S6 opcode dead path cleanup** (1 level-3 task)

| Level 2                              | Level 3                                                                                                                                                                                                                                                                                                                                                         | Acceptance                                                                                                                                             |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S2 Delete pure placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladders / old syntax probes / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: two `_ => todo!()` at `lib.rs:568` / `:582` changed to meaningful fallback output**                                                                                                                   | `cargo test` passes; `clippy -D warnings` no new warnings                                                                                              |
| S3 Empty design documents            | C1+C2 deleted and rewritten to point to `src/frontend/module/` (~2,670 lines); C3 rewritten boundary table; **synchronously clean up `config.js:259-268`**                                                                                                                                                                                                      | No 404 on docs site; `check-docs-truth.py` passes                                                                                                      |
| S4 RFC status corrections            | C4 RFC-018 moved back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 `TRACKING.md` add "Implementation Status" column (D36)                                                                                                                                                                                                                 | `check_tracking.py` exit code 0 (**do not manually edit TRACKING.md**)                                                                                 |
| S5 Reduce `pub` visibility           | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be preserved**)                                                                                                                                                                                                                                                                              | Compiler can point out the real dead code                                                                                                              |
| S6 opcode dead path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete, opcode value recycling already landed in 7e.1 with version bump); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes `op` field); **F7+F8 opcode generation-time gate** (extend `tools/code-tables` opcode extractor ~30 lines + 5-place fact expression single-sourcing + `size()` full opcode comparison test) | 5-place opcode fact expression converges to 1 authoritative source; `size()` comment inconsistent with actual encoding (`bytecode.rs:2181-2182`) fixed |

## Implementation Points: Dependencies and Parallelization

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 bleeding channel (2026-10-05 ruling)**: P3's prerequisites narrow to **2.3.2 + 2.4.1 +
> 2.4.3** (multi-file corpus layer + two red oracles). The rest of P2 (2.1 IR verifier, 2.2
> snapshots, 2.3.1/2.3.3 diff and performance baseline) can proceed in parallel with P3. The
> bleeding of correctness vulnerabilities is not blocked by oracle infrastructure construction; but
> **P4 must still wait for P2 to complete fully** — the unified Driver needs all three oracle layers
> in place.

| Constraint                | Reason                                                                                                                                                    |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1                   | The three prohibitions constrain every action in P1 (1.2.x is exactly adding `mod` declarations to `mod.rs`)                                              |
| P1 → P2                   | Resurrecting tests will change test count and corpus baseline                                                                                             |
| P2 (after narrowing) → P3 | Vulnerability oracles must **be written red first** (2.4.1/2.4.3) + multi-file corpus layer (2.3.2) must exist for the red tests to have somewhere to run |
| P3 → P4                   | Fix bugs first, then refactor; reverse order will let bugs be solidified into "established behavior" by the stage table                                   |
| P4 → P5                   | `include!` refactor and checker split touch the same file                                                                                                 |
| P5 → P6                   | Consecutive changes to the same file must be separated, otherwise regressions cannot be bisected                                                          |
| P6 → P7 / P8              | Without first converging type representation, the new IR will grow into a third set of representations                                                    |
| P7/P8 → P9                | 9.5's snapshot baseline and 9.3's `pub(crate)` leakage count only take their final form after P7/P8                                                       |

### Parallelizable Groups

| Group                              | Members                                                                        | Basis                                                                                                                                                                                                                                            |
| ---------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **A (only one authorized by RFC)** | P7 ∥ P8                                                                        | Disjoint file sets: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                                    |
| B                                  | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                  | Each independently adds `mod`, revert file by file                                                                                                                                                                                               |
| C                                  | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                | Three oracle layers independent of each other                                                                                                                                                                                                    |
| D                                  | 4.4 (proof layer wrap-up) ∥ entire P6 line                                     | `02` explicitly states S5 "should walk independently from S1-S4"                                                                                                                                                                                 |
| E                                  | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                          | All mutually independent after 4.1 completes                                                                                                                                                                                                     |
| F                                  | 9.3 ∥ 9.9 ∥ 9.10 (three scripts introduced in P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                                                                                                                                                                       |
| G                                  | S3 ∥ S4 ∥ S5                                                                   | Disjoint file sets (verified 2026-10-05: S3 touches `docs/src/design/check/` + `config.js`; S4 touches `docs/src/design/rfc/` + `check_tracking.py` generator; S5 touches two `pub` in `src/frontend/core/typecheck/`; no pairwise intersection) |

**Cannot parallelize (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5
same-batch files); 4.3.2 ∥ 9.2 (already unified to P4).

**Internal document new constraints**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial; 7a→7b→7c→7d
strictly serial, and 7b's `Phi` arm must precede 7d.

## Key Decisions and Rationale

- **P0 as an independent phase rather than merged into P9** — gates are "post-hoc checks",
  regulations are "pre-hoc judgment". P9's scripts cannot prevent "should have refactored but only
  patched"; only D0–D4's review checklist can.
- **P9 as the sole authoritative CI script list** — the original gap G3's three parallel lists made
  closing impossible. After this table merges, 10 scripts are each assigned to one phase, with
  report-only and hardening timing explicitly queryable.
- **P5's steps are filled in by this document rather than merged into P4** — P4 is already the
  largest single point of risk; adding the checker split on top would exceed rollback granularity.
- **5.3.2 uses "zero caller changes" as hard acceptance** — the split must be independently
  revertible.

## Known Limitations and Risks

- **1.3.x (fix exposed defects) has no regression oracle** — only says "fixed defects should not be
  rolled back", lacking record requirements for "what was fixed". **Execution convention**: every
  1.3.x fix's commit message must reference the name of the test being fixed and the defect
  description, as a post-hoc bisection basis.
- ~~**7b.1 granularity too coarse**~~ **Fixed (2026-10-05)**: split into 7b.1a / 7b.1b two exclusive
  commits per DoD 6.
- **8.2.1 granularity too coarse** — ~89 rename references; missing a change **silently changes
  associativity**.
- ~~**D17 / D20 have no corresponding level-3 task**~~ **Fixed (2026-10-05)**: D17 lands as 7e.1,
  D20 lands as 4.5.1/4.5.2; D32/D34's opcode recycling merges into 7e.1, deletion action stays in
  P10 S6.
- ~~**`2315` / `2341` data loss defects originally "independent issue"**~~ **Fixed (2026-10-05)**:
  absorbed as 7e.2 / 7e.3. Exception table (throw/try) is core language semantics, must not be left
  as implementation legacy.
- ~~**E2 (`lib.rs:568/582` `todo!()`) has no phase attribution**~~ **Fixed (2026-10-05)**: merged
  into P10 S2.
- ~~**F7 (opcode gate extractor) attribution ambiguous**~~ **Fixed (2026-10-05)**: merged into P10
  S6's generation-time gate task.

## Conflict Registry: All Resolved

> **The original "Conflict Registry" C1–C8 and each document's open issues have all converged into
> [RFC-039 Decision Registry](../rfc/draft/039-compiler-architecture.md) (D1–D50).** No open items
> remain in this document. One-to-one correspondence:

| Original # | Topic                                                      | Ruling                                                                                                                                                  |
| ---------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1         | `precedence_inline.rs` resurrected in P1, deleted in P8    | **Resurrect** (P1's premise is resurrecting tests with real assertions); P8's 8.6 deletes the file along with Pratt                                     |
| C2         | "Grammar-driven" commitment vs table-lookup                | **Done fully**: adopt LALRPOP, see `05` "Syntax: Complete Grammar-Driven"                                                                               |
| C3         | C6 category definition conflicts with S1 classification    | **S1 merged into C6** (resurrection needs no equivalence oracle, only regression tests); C6 definition expanded to "pure deletion or pure resurrection" |
| C4         | `test_release_plan_spans_consumed` whitelist               | **No whitelist**, difference set must be empty; if it cannot be empty, the `ReleasePlan` contract is defective → change to `PlanId` (D20/D41)           |
| C5         | `Aggregation` two function internal differences unverified | **P4's 4.2.7 has listed it as must-verify**, confirm line by line during implementation                                                                 |
| C6         | wasm playground fix needs P4's `ProgramKind`               | **wasm path vulnerability fix merged into 4.2.8**; P3 only handles three native entries                                                                 |
| C7         | `method_def_ordinals` attribution                          | **Attributed to 04** (P7 batch c), see D21                                                                                                              |
| C8         | `upvalue_count` fix and `.42` version number               | **Fix + `VERSION` upgrade from 4 to 5** (format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`), see D17                    |

## See Also

- [RFC-039 Body](../rfc/draft/039-compiler-architecture.md) — Sole authority for phase sequence,
  DoD, global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 oracle category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0 detailed design and D0–D4 decision
  procedure
- [01-routing.md](01-routing.md) — Target directory structure (post-task-completion form)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — Design basis for each phase
