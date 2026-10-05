# Multi-level Work Breakdown Structure (WBS)

> **Companion Design Document**. This document is a companion to
> [RFC-039: Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). It
> expands the RFC's phase sequence into **independently committable, independently verifiable**
> three-level tasks.
>
> The phase ordering and acceptance gates are governed solely by the body of RFC-039; this document
> only breaks them down to an executable granularity and logs conflicts that have not yet been
> adjudicated.

## Positioning and Scope

| Level       | Count       | Meaning                                                                             |
| ----------- | ----------- | ----------------------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Phases, one-to-one with the phase sequence in RFC-039                               |
| **Level 2** | 45          | Task groups, drawn from the "Implementation Notes" sections of the design documents |
| **Level 3** | 122         | Independently committable actions                                                   |

**This document covers**: task breakdown, dependency relations, parallel groupings, and binding to
acceptance criteria.

**This document does not cover**: the design rationale of tasks (see `02`–`08`), or the
justification of phase ordering (see RFC-039).

## Current State: Gaps Discovered During Breakdown

While consolidating the "Implementation Notes" from the 8 design documents into this table, **5 hard
gaps in the documentation system itself** were found. All have been converged:

| #   | Gap                                                                                                                                                                                                                                                                                                                                                              | Resolution                                                                                                                                                        |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no construction steps**. The RFC defines P5 (checker splitting) and acceptance (C1 zero-diff), but none of the 7 documents describes how to perform this split; the only mention in the entire batch is the single line `annotations.rs` in the directory tree of `01-routing.md`                                                                       | P5 steps are filled in by this document (see below)                                                                                                               |
| G2  | **`include!` refactoring: three callouts, zero landings**. The RFC lists it under P4; `02` assigns the cross-reference to itself; `06` S7 also lists it. But the change list in `02` (6 additions + 20 modifications) **does not include `semantic_tokens.rs`**                                                                                                  | Construction steps are assigned to §P5 5.1 of this document (after P4 completes); references in the RFC and 06 now point here                                     |
| G3  | **Three coexisting CI script lists**. The `RFC` lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists 4 different checks (all named differently); `04` additionally requires `check-synth-boundary.py`. After deduplication **at least 9 new scripts, with no unified list**; `check-obligations.py` is double-assigned to both P4 and P9 | Unified list in §P9 of this document (10 scripts, each assigned to one phase); all other documents refer to that table                                            |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it among the "1005 lines to revive", but B6 in the same document says it must be deleted along with the `Precedence` enum, while `05` advocates deletion                                                                                                                                         | **Revive in P1, delete in P8 along with Pratt** (resolution logged as C1) — first restore CI coverage, then the deletion happens with the dead ladder in P8's 8.6 |
| G5  | **Conflicting L2 goal statement**. The RFC and this directory's index say "grammar-driven syntax", but `05` self-assesses: "**did not adopt true grammar-driven (LALRPOP), only achieved declarative table lookup**"                                                                                                                                             | Converged to **full LALRPOP grammar-driven** (`05` §Syntax: Full Grammar-Driven); the G7/G8 acceptance gates are executed against that commitment                 |

## Target Design: Phase Sequence

```
P0  Repository Maintenance Mechanism and Code Placement Rules    08        ← Newly added, establish rules first
P1  Revive Orphan Tests and Fix Defects                           06 §S1
P2  Build Equivalence Oracle Baseline                             07 all
P3  Fix Correctness Bugs (Minimal Solution)                       02 §S3-vulnerability half
P4  Phase Contract and Unified Driver                             02
P5  checker In-file Splitting                                    02 + filled in by this doc
P6  Type Representation Unification                              03
P7  Intermediate Representation SSA-ization                       04      } Parallelizable
P8  Frontend Paradigm Change                                     05      }
P9  Anti-regression Gates (incl. Unified Script List)             01 §Anti-regression + 08 §Machine-checkable Rules
P10 Remaining Cleanup and Status Corrections                      06 §S2/S3/S4/S5/S6
```

**Rationale for the newly added P0**: The three prohibitions (no fabrication / no endless padding /
should have refactored but applied a patch) constrain every action in P1–P10. The rules must be
established before construction begins; otherwise, P1's test revival would already be adding `mod`
declarations to `tests/mod.rs` — which is precisely how the G4 conflict arose.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [ ] **0.1 Codify the rules** (3 level-3 tasks; touchpoints in 0.1.2 and rule extraction in 0.1.3
      were completed ahead of schedule on 2026-10-05, leaving 0.1.1 review of CONTRIBUTING body)
- [ ] **0.2 Gate implementation** (3 level-3 tasks)
- [ ] **0.3 Gate rollout to tables** (1 level-3 task)

| Level 2                    | Level 3                                                                                                                                                                                                                                                                                                                                                  | Prerequisite | Acceptance                                                                                                                                                                              |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Codify the rules       | 0.1.1 Write D0–D4 + three prohibitions into `CONTRIBUTING.md`; responsibility categories reference the directory responsibility table in `01-routing.md`                                                                                                                                                                                                 | —            | Pass manual review                                                                                                                                                                      |
|                            | 0.1.2 Implementer touchpoints landed (**completed 2026-10-05**): new `AGENTS.md` (repo root entry) + `compiler-architecture/HOWTO.md` (pre-construction self-check + D3 patch judgment); PR template adds required block "Responsibility Attribution and Decision Procedure" (D44); `CONTRIBUTING.md` adds section "Code Placement and Change Procedure" | —            | Landed; review verifies the four touchpoints                                                                                                                                            |
|                            | 0.1.3 Rule body extracted (**completed 2026-10-05**): new `docs/src/dev/coding-rules.md` (precise rule description, long-lived, not archived with RFC-039); 08 adds migration header note converted to diagnostic record; all four touchpoints now point to coding-rules                                                                                 | —            | Landed                                                                                                                                                                                  |
| 0.2 Gate implementation    | 0.2.1 `check-concepts.py` (Prohibition One A/B/C/D)                                                                                                                                                                                                                                                                                                      | 0.1.1        | **Must report, on unmodified code, 3 sets of operator enums and 2 sets of parallel type representations (the `ir::Type` alias counts too)**; report-only at this phase, hard-fail in P9 |
|                            | 0.2.2 `check-fanout.py` (Prohibition Three A/B/C)                                                                                                                                                                                                                                                                                                        | 0.1.1        | **Deliberately adding a 6th entry-style wiring must fail**; report-only at this phase, hard-fail in P9                                                                                  |
|                            | 0.2.3 `check-boundary.py` (Prohibition Two criterion C)                                                                                                                                                                                                                                                                                                  | 0.1.1        | **Deliberately adding an L2→L3 reverse `use` must fail**; record the initial value of `pub(crate)` leakage count                                                                        |
| 0.3 Gate rollout to tables | 0.3.1 Extend `tools/code-tables` to cover opcode / type tables                                                                                                                                                                                                                                                                                           | 0.1.1        | Depends on P6 T1; rolled up independently                                                                                                                                               |

> **No size baseline task**. The 2026-10-03 decision cancels all line-count / volume gates; the size
> problem is solved by responsibility separation (see `08` Prohibition Two criterion A, manual
> judgment). **0.2.1 is this phase's core acceptance**: if the gate does not report the known
> parallel representations, the criterion is broken and the gate is a sham.

### P1 Revive Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Notes S1 + [05](05-frontend-paradigm.md) §Test
Reconstruction

- [ ] **1.1 Revive `lexer/tests/`** (4 level-3 tasks)
- [ ] **1.2 Revive 4 spots in living directories** (1 level-3 task)
- [ ] **1.3 Fix exposed defects** (2 level-3 tasks)

| Level 2                                  | Level 3                                                                                                                                                    | File:Line                  | Prerequisite | Acceptance                                                            |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | --------------------------------------------------------------------- |
| 1.1 Revive `lexer/tests/`                | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                        | after `lexer/mod.rs:106`   | —            | Test count rises                                                      |
|                                          | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                                | `lexer/tests/mod.rs:15-25` | 1.1.1        | **159 lines still won't run without this**                            |
|                                          | 1.1.3 Delete 7 empty shell files + their `mod` declarations                                                                                                | 7 files in `lexer/tests/`  | 1.1.1        | C6 has no criterion                                                   |
|                                          | 1.1.4 Delete the `pub use ...::*;` re-export block                                                                                                         | `tests/mod.rs:28-38`       | 1.1.3        | This block is exactly what masked the empty shells                    |
| 1.2 Revive 4 spots in living directories | 1.2.1–1.2.4 respectively wire up `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` | each parent module         | —            | Test count rises; **1.2.1 conflicts with 8.0.1, see conflict log C1** |
| 1.3 Fix exposed defects                  | 1.3.1 Overflow paths in 4 radix scanners                                                                                                                   | `literals.rs`              | 1.1.x        | `cargo test` green                                                    |
|                                          | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                       | `literals.rs`              | 1.1.x        | Same as above                                                         |

**Acceptance**: Test count rises; **real defects are expected to be exposed**, so fix time must be
reserved. Revert file by file, but **fixed defects must not be rolled back**.

### P2 Build Equivalence Oracle Baseline

Source: [07](07-equivalence-oracle.md) all (this document has no "Implementation Notes" section,
derived from structure)

- [ ] **2.1 IR static verifier** (3 level-3 tasks)
- [ ] **2.2 Normalized snapshots** (2 level-3 tasks)
- [ ] **2.3 Corpus diffing** (3 level-3 tasks)
- [ ] **2.4 Bug-specific oracles** (3 level-3 tasks)
- [ ] **2.5 Regression gate** (1 level-3 task)

| Level 2                  | Level 3                                                                                                                                                   | Prerequisite | Acceptance                                                                              |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------- |
| 2.1 IR static verifier   | 2.1.1 `verify.rs` dual mode `verify_loose` / `verify_ssa`                                                                                                 | —            | C4                                                                                      |
|                          | 2.1.2 **`verify_loose` runs green on existing non-SSA IR**                                                                                                | 2.1.1        | **Hard gate, entry condition for P7 batch a**                                           |
|                          | 2.1.3 Implement 7 invariants (dominance / single definition / Phi consistency / jump targets / global out-of-bounds / type consistency / inner isolation) | 2.1.1        | C4                                                                                      |
| 2.2 Normalized snapshots | 2.2.1 Normalization tool (strip Span / rename temps / relativize slots / sort predecessors)                                                               | —            | C1/C3/C5                                                                                |
|                          | 2.2.2 Snapshot check-in + manual review process (`src/middle/core/tests/snapshots/`)                                                                      | 2.2.1        | C1/C3/C5                                                                                |
| 2.3 Corpus diffing       | 2.3.1 Diff framework + 293-corpus baseline                                                                                                                | —            | Diagnostics / exit code / stdout item by item                                           |
|                          | 2.3.2 **Multi-file corpus layer** (new `tests/yaoxiang-multifile/`, project fixtures with `yaoxiang.toml` — resolution D48)                               | —            | **Unconditionally required (D40); the only executable behavioral oracle source for P4** |
|                          | 2.3.3 C4 behavioral diff mandatory coverage list (10 semantic categories) + performance baseline (criterion smoke benchmarks)                             | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                                  |
| 2.4 Bug-specific oracles | 2.4.1 `test_multifile_proof_obligation_not_dropped`                                                                                                       | 2.3.2        | **Must be red first**                                                                   |
|                          | 2.4.3 `test_no_silent_pass_on_unproven`                                                                                                                   | —            | Catches `checker.rs:5179/5318/5448`                                                     |
|                          | 2.4.4 `test_release_plan_spans_consumed`                                                                                                                  | —            | **No whitelist, difference set must be empty (D41)**                                    |
| 2.5 Regression gate      | 2.5.1–2.5.4 see P9 unified list                                                                                                                           | 2.1–2.3      | See P9                                                                                  |

> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: both reference the `Program` / `Obligations` types that only exist in P4
> and cannot compile in P2. The former becomes 4.1.4 in P4 (no double registration); the latter's
> `#[ignore]` red skeleton merges into 4.3.1.

### P3 Fix Correctness Bugs (Minimal Solution)

Source: [02](02-stage-contract.md) §Implementation Notes S3 vulnerability fix half

- [ ] **3.1 `proof_calls` consumer side** (2 level-3 tasks)
- [ ] **3.2 Second silent-drop point** (1 level-3 task)
- [ ] **3.3 Panic to diagnostic** (1 level-3 task)

| Level 2                         | Level 3                                               | File:Line                                | Prerequisite | Acceptance                          |
| ------------------------------- | ----------------------------------------------------- | ---------------------------------------- | ------------ | ----------------------------------- |
| 3.1 `proof_calls` consumer side | 3.1.1 Make `proof_calls` private + the only getter    | `types.rs:29`                            | 2.4.1        | Vulnerability oracle turns green    |
|                                 | 3.1.2 Fill in the three consumer points               | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1        | C2                                  |
| 3.2 Second silent-drop point    | 3.2.1 `Unproven` empty match arm produces diagnostic  | `checker.rs:1303-1314`                   | 2.4.3        | `test_no_silent_pass_on_unproven`   |
| 3.3 Panic to diagnostic         | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic | `predicate.rs:34-36`                     | —            | No longer panics when Z3 is missing |

**Acceptance**: The vulnerability oracles turn green, and **deliberately reverting the fix must turn
them red again**. **Risk**: Adding a new E4018 in the multi-file path is a breaking change.

> **Hemostasis channel (adjudicated 2026-10-05)**: P3 does not need to wait for all of P2 to
> complete. The only hard prerequisite for 3.1/3.2 is **2.3.2 (multi-file corpus layer) +
> 2.4.1/2.4.3 (two red oracles)**; 2.1/2.2/2.3.1/2.3.3 (IR verifier, snapshots, single-file diff,
> performance baseline) can proceed in parallel with P3. The hemostasis of correctness bugs (silent
> pass on `Sorted(3)`) must not be blocked by snapshot infrastructure. 3.3 (panic to diagnostic) has
> no prerequisites and can land at any time.

### P4 Phase Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Notes S1–S5. **This phase's changes and P3's
fixes must be in two separate commits.**

- [ ] **4.1 Declarative stage table** (5 level-3 tasks)
- [ ] **4.2 Entry consolidation** (8 level-3 tasks)
- [ ] **4.3 Obligation ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm wrap-up** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ization (D20)** (2 level-3 tasks)

| Level 2                                       | Level 3                                                                                                                                       | Prerequisite | Acceptance                                                                                                                                            |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative stage table                   | 4.1.1 `stage.rs` (`Stage` variants + `Stage::ALL` + `StageScope`)                                                                             | P0           | `test_program_stage_coverage`                                                                                                                         |
|                                               | 4.1.2 `program.rs` / `unit.rs`                                                                                                                | 4.1.1        | Same as above                                                                                                                                         |
|                                               | 4.1.3 `Pipeline::run` rerouted through Driver (`pipeline.rs:141-227`)                                                                         | 4.1.2        | **Single-file diagnostic set and exit code byte-for-byte identical**                                                                                  |
|                                               | 4.1.4 Phase coverage assertion                                                                                                                | 4.1.3        | C2                                                                                                                                                    |
|                                               | 4.1.5 Zero-diff across the full corpus                                                                                                        | 4.1.4        | Diagnostics / exit code / stdout all zero-diff (293 corpora)                                                                                          |
| 4.2 Entry consolidation                       | 4.2.1 Slim `compile_project` into a Program constructor (`orchestrator.rs:99-237`)                                                            | 4.1.5        | Diagnostic sets from the four entries identical after unification                                                                                     |
|                                               | 4.2.2 `check_project` (`:273-399`)                                                                                                            | 4.2.1        | Same as above                                                                                                                                         |
|                                               | 4.2.3 `check_source_in_project` (`:450`)                                                                                                      | 4.2.2        | Same as above                                                                                                                                         |
|                                               | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                                     | 4.2.3        | Same as above                                                                                                                                         |
|                                               | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                                | 4.2.1        | `check` consistent inside and outside a project                                                                                                       |
|                                               | 4.2.6 Delete LSP manual phase sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                                | 4.2.1        | LSP and CLI diagnostic sets identical                                                                                                                 |
|                                               | 4.2.7 `Aggregation` parameter-driven two-way choice (`orchestrator.rs:486-494`)                                                               | 4.2.1        | **Verify the differences between the two functions line-by-line during implementation** (conflict log C5 adjudicated as a required verification item) |
|                                               | 4.2.8 wasm rerouted through `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                                     | 4.2.1        | See C6                                                                                                                                                |
| 4.3 Obligation ledger                         | 4.3.1 `obligations.rs` + `assert_drained()` (new; includes the `#[ignore]` red skeleton first)                                                | 4.1.3        | `test_obligations_drained` turns green                                                                                                                |
|                                               | 4.3.2 Obligation diagnostic W→E escalation                                                                                                    | 4.3.1        | Manual review of every newly added E                                                                                                                  |
| 4.4 Proof layer and wasm wrap-up              | 4.4.1 `layers/README.md` layer order changed to actual order                                                                                  | —            | **Walks independently of 4.1–4.3**                                                                                                                    |
|                                               | 4.4.2 `default_solver()` changed to `&'static` singleton (`proof/smt/backend.rs:67-72`)                                                       | —            | Cache hit rate is observable                                                                                                                          |
|                                               | 4.4.3 Add warning to the degradation path (`checker.rs:1283-1293`)                                                                            | —            | —                                                                                                                                                     |
| 4.5 Cross-layer contract PlanId-ization (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (`layers/ownership.rs:31` allocation on producer side + `ir_gen.rs:1942` matching on consumer side) | 4.3.1        | `test_release_plan_spans_consumed` difference set empty (D41)                                                                                         |
|                                               | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                            | 4.5.1        | C2; silent failures from span mismatch drop to zero                                                                                                   |

### P5 checker In-file Splitting — Filled in by This Document (Original Gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (`collect_used_in_type`'s
`pub(crate)` path must not change)

- [ ] **5.1 `include!` → real `mod`** (2 level-3 tasks)
- [ ] **5.2 Split out `refinement`** (2 level-3 tasks)
- [ ] **5.3 Split out `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                                       | Prerequisite | Acceptance                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------------------- |
| 5.1 `include!` → real `mod` | 5.1.1 Change `checker.rs:5618`'s `include!` to a real `mod` declaration                                                                                                                                                                                       | P4 complete  | C1 zero-diff; `semantic_tokens.rs` acquires module identity       |
|                             | 5.1.2 Add `use super::*` or item-by-item imports (the `include!` era inherited scope implicitly)                                                                                                                                                              | 5.1.1        | `cargo build` green                                               |
| 5.2 Split out `refinement`  | 5.2.1 Move out the refinement block (currently `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                           | 5.1.1        | C1 zero-diff                                                      |
|                             | 5.2.2 `collect_refined_binding_checks` signature change: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                          | 5.2.1        | C1; only call site is `1235`                                      |
| 5.3 Split out `annotations` | 5.3.1 Move out annotation validation (currently `3441-3661`, including `is_predicate_head`)                                                                                                                                                                   | 5.2.2        | C1 zero-diff                                                      |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) remains `pub(crate)` and lives under the `checker` module path** → add `pub(crate) use annotations::*;` at the top of `checker.rs`, so that the import in `inference/statements.rs:16` is **unchanged** | 5.3.1        | **Zero caller-side changes** is the hard acceptance for this step |

**Hard constraint**: `collect_used_in_type` is used across modules by `inference/statements.rs:16`.
The approach is `pub(crate) use` re-export, making the split a **pure relocation**.

**Only split the substantiated large blocks.** Do not split modules that are only 200–400 lines —
that adds navigation cost, reduces cohesion, and yields zero decoupling benefit. The other
sub-modules (signatures / type_defs / imports) are split later when the file grows back to 1,500
lines.

### P6 Type Representation Unification

Source: [03](03-type-unification.md) §Implementation Notes phases 0–6, criterion C3. **Phases 0→6
are strictly serial.**

- [ ] **6.0 Gate first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead variant deletion** (2 level-3 tasks)
- [ ] **6.4 parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variants** (2 level-3 tasks)
- [ ] **6.6 Gate turned hard** (1 level-3 task)
- [ ] **6.7 Directory renaming (D1, pure relocation batch)** (1 level-3 task)

| Level 2                                            | Level 3                                                                                                                                                                                             | File:Line                                                                     | Acceptance                                                                                                             |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 6.0 Gate first                                     | 6.0.1 New `tools/type-tables`, implement T1/T2/T3, **T1 reports only, does not fail**                                                                                                               | new crate                                                                     | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge reconstruction points listed separately** |
|                                                    | 6.0.2 Take the full baseline on unmodified code                                                                                                                                                     | —                                                                             | No code changes                                                                                                        |
| 6.1 Name normalization                             | 6.1.1 Count occurrences of non-canonical type names in the corpus                                                                                                                                   | `tests/yaoxiang/`                                                             | Determines whether synonyms can be deleted directly                                                                    |
|                                                    | 6.1.2 `from_builtin_name` cuts synonyms → lexer alias table                                                                                                                                         | `mono.rs:618-643`                                                             | C3                                                                                                                     |
|                                                    | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                | `types.rs:162-165`                                                            | **Must be verified line by line; cannot be assumed equivalent**                                                        |
| 6.2 Bytecode type unification                      | 6.2.1 Change 3 sites to `MonoType`                                                                                                                                                                  | `bytecode.rs:812/814/854`, `image.rs:44`                                      | C3 + targeted `dump_bytecode` compare                                                                                  |
|                                                    | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                            | `bytecode.rs:2352-2390`                                                       | Same as above                                                                                                          |
|                                                    | 6.2.3 `type_table` element type changed to `MonoType`                                                                                                                                               | `image.rs:44`                                                                 | Same as above                                                                                                          |
| 6.3 Dead variant deletion                          | 6.3.1 Delete 11 variants + clear match arms + fallback                                                                                                                                              | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                                   | C3 + `cargo build` green                                                                                               |
|                                                    | 6.3.2 **Must be in the same batch as 6.3.1** rewrite the reverse bridge                                                                                                                             | `passes/mono/function.rs:507-529` (`mono_to_ast_type` reconstructs 7 of them) | Targeted generic-substitution compare, otherwise **silently changes behavior**                                         |
| 6.4 parser data flow                               | 6.4.1 New `probe.rs`                                                                                                                                                                                | new                                                                           | C3                                                                                                                     |
|                                                    | 6.4.2 Modify `parser_state.rs:46-58`                                                                                                                                                                | —                                                                             | Same as above                                                                                                          |
|                                                    | 6.4.3 Delete `operator_interfaces::spec()` calls (**2 sites**: `declarations.rs:509`, `ast.rs:930`)                                                                                                 | —                                                                             | Same as above                                                                                                          |
|                                                    | 6.4.4 Merge the two `name_used_as_type*`                                                                                                                                                            | `ast.rs:847` + `declarations.rs:42`                                           | Same as above                                                                                                          |
|                                                    | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) wired through `NameKind::Builtin`                                                                                                                          | —                                                                             | Same as above                                                                                                          |
| 6.5 AST dead variants                              | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                          | `ast.rs:37-43` + 4 consumer sites + 2 exhaustive arms                         | `tests/integration/` 18 modules green (**split into 2 commits**)                                                       |
|                                                    | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                     | `ast.rs:241-249`                                                              | **Note semantic risk**                                                                                                 |
| 6.6 Gate turned hard                               | 6.6.1 T1 changed from warning to `panic!`                                                                                                                                                           | `tools/type-tables`                                                           | **Deliberately introducing a zero-construction variant must fail**                                                     |
| 6.7 Directory renaming (D1, pure relocation batch) | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; `parser/ast.rs` move to top-level `ast/`; bytecode domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` move to `middle/bytecode/` | whole repo                                                                    | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline does not regress                             |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA-ization

Source: [04](04-ssa.md) §Implementation Notes batches a–d, criterion C4, **strictly serial within a
batch**.

- [ ] **7a Cut off multiple definitions** (3 level-3 tasks)
- [ ] **7b SSA form switching** (6 level-3 tasks; 7b.1 already split into 1a/1b per DoD 6)
- [ ] **7c Implicit contract explicitation** (3 level-3 tasks)
- [ ] **7d Final pass** (2 level-3 tasks)
- [ ] **7e `.42` data loss fix** (3 level-3 tasks; 2315/2341 incorporated from "independent issue",
      2026-10-05)

| Level 2                            | Level 3                                                                                                                                                       | Prerequisite                        | Acceptance                                   |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------- |
| 7a Cut off multiple definitions    | 7a.1 Delete 3 statement-level recycles                                                                                                                        | **2.1.2 `verify_loose` runs green** | C4                                           |
|                                    | 7a.2 Replace 6 save/restore sites with RAII guards                                                                                                            | 7a.1                                | C1 snapshot zero-diff                        |
|                                    | 7a.3 Caliber unification (`1798` vs `2001`)                                                                                                                   | 7a.2                                | C1                                           |
| 7b SSA form switching              | 7b.1a `ir.rs` structural change #1-2 (`Operand` variant convergence + `Instruction::Phi` introduction)                                                        | 7a.3 + **P6 complete**              | C4; exclusive commit                         |
|                                    | 7b.1b `ir.rs` structural change #3-5 (`FunctionIR` / `BasicBlock` / type annotation fields)                                                                   | 7b.1a                               | C4; exclusive commit                         |
|                                    | 7b.2 `next_temp_reg` changed to `Operand::Value`                                                                                                              | 7b.1                                | Same as above                                |
|                                    | 7b.3 `translator.rs` adds `Phi` arm                                                                                                                           | 7b.1                                | C4 + `.42` round-trip test                   |
|                                    | 7b.4 Linear scan allocator (within the u8/255 slot model)                                                                                                     | 7b.3                                | Same as above (+500~1000 lines)              |
| 7c Implicit contract explicitation | 7c.1 `synth.rs` boundary + boundary check script                                                                                                              | 7b.4                                | C4 + new script in CI                        |
|                                    | 7c.2 Span consumption counting                                                                                                                                | 7c.1                                | Consumption count **zero triggers**          |
|                                    | 7c.3 `method_def_ordinals` made read-only                                                                                                                     | 7c.1                                | Attribution uncertain (C7)                   |
| 7d Final pass                      | 7d.1 Split `generate_call_expr_ir` into `CallArgs` + 6 `emit_*`                                                                                               | 7c.x                                | C1 snapshot zero-diff                        |
|                                    | 7d.2 "Fill 0" fallback changed to return diagnostic                                                                                                           | 7d.1                                | C4                                           |
| 7e `.42` data loss fix             | 7e.1 `upvalue_count: 0` correction (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + already-allocated unused opcode reclamation (D32/D34 along with version bump) | 7d.x                                | C4 + `.42` round-trip test; exclusive commit |
|                                    | 7e.2 `exception_handlers` write-out (`bytecode.rs:2315` — throw/try are core language semantics, **must not be left for later implementation**)               | 7e.1                                | `.42` direct-run exception case diff         |
|                                    | 7e.3 `globals` write-out (`bytecode.rs:2341`)                                                                                                                 | 7e.1                                | `.42` direct-run global variable case diff   |

**Expectation management**: Net code increase of **900–1600 lines**. If "code becomes shorter" is
taken as the success criterion at project kickoff, this phase will be judged a failure.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Notes phases 0–4, criterion C5.

- [ ] **8.0 Dead ladder cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexical convergence** (2 level-3 tasks)
- [ ] **8.3 Build LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual-parser diff** (1 level-3 task)
- [ ] **8.5 Switch over** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 `parse_assign_after_target` split** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)

| Level 2                               | Level 3                                                                                                | Prerequisite                     | Acceptance                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ | -------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead ladder cleanup (C6)          | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                        | **Conflicts with 1.2.1, see C1** | C6 has no criterion                                                                                              |
|                                       | 8.0.2 Name bare magic numbers (`(6,7)` / `(11,1)` / `12`)                                              | —                                | Same as above                                                                                                    |
| 8.2 Lexical convergence               | 8.2.1 Merge 4-radix scanners + merge 3 escape sites + merge multi-line strings                         | —                                | C5; `literals.rs` shrinks by ~500 lines                                                                          |
|                                       | 8.2.2 Eliminate f-string nested compilation                                                            | 8.2.1                            | **span necessarily changes**, baseline must be taken first                                                       |
| 8.3 Build LALRPOP grammar             | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                          | 8.2.x                            | `cargo build` passes; can produce AST for 293 corpora (**no diffing**)                                           |
|                                       | 8.3.2 Action code `grammar/actions.rs` (one function per 22 `Expr` variants)                           | 8.3.1                            | Same as above                                                                                                    |
|                                       | 8.3.3 Error productions (preserve `Expr::Error` / `StmtKind::Error` placeholder semantics)             | 8.3.2                            | Same as above                                                                                                    |
| 8.4 **Dual-parser diff**              | 8.4.1 Both parsers each run 293 corpora + `src/std/tests`, AST normalized then **bit-by-bit compared** | 8.3.3                            | **All legal-program ASTs equivalent — this is the equivalence proof for the entire grammar migration**           |
| 8.5 Switch over                       | 8.5.1 `parse()` changed to call LALRPOP, Pratt kept as `parse_legacy()`                                | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span identical item by item (**no relaxation**, no C5′ exists) |
| 8.6 Delete Pratt                      | 8.6.1 Delete `nud.rs` (1326) + `led.rs` (451) + two sets of BP ladders + ~89 references                | 8.5.1                            | Full corpus green; `git grep BP_` zero hits                                                                      |
|                                       | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally obsolete)                                   | 8.6.1                            | Same as above                                                                                                    |
| 8.7 `parse_assign_after_target` split | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                                 | 8.6.x                            | This function only dispatches                                                                                    |
| 8.8 Test completion                   | 8.8.1 Add associativity cases (**red then green**)                                                     | 8.2.x                            | C5                                                                                                               |
|                                       | 8.8.2 Add 4 categories of literal error paths                                                          | —                                | Cover untested paths                                                                                             |
|                                       | 8.8.3 Add `pratt/tests/mod.rs` wiring self-check assertion                                             | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6, changed to `parser/tests/`**)            |

**Risk**: 8.4 dual-parser diff is **an unavoidable equivalence gate**. If it produces inequivalence,
it must be resolved before 8.5 switching, not papered over after switching via C5′. If 8.6's ~89 BP
reference migrations miss a change, it will **silently change associativity** (no error, only the
parse result changes).

> **8.3–8.6 depend on D0/D2 from `08`**: if the grammar file is placed outside `parser/`, or the
> action code back-references `sema`, it will be caught at `check-boundary.py`.

### P9 Anti-regression Gates (Unified Script List — Filled in by This Document for Original Gap G3)

Source: [01](01-routing.md) §Anti-regression Mechanism + [08](08-maintenance-mechanism.md)
§Machine-checkable Rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts join CI in their
"introduction phase"; those marked report-only are uniformly turned hard in P9 (changed from
"report" to "fail") — existing violations (parallel representations, disambiguation aliases, etc.)
need P6/P8 to clear to zero; turning hard too early will make CI red long-term and the gate bypassed
manually; when P9 turns hard, the gates should be exactly all green. P2's 2.5, P4's 4.3.2, and P7's
7c.1 all refer to this table and do not establish their own lists.

- [ ] **9.1–9.10 ten scripts join CI in their introduction phases, uniformly turned hard in P9**
      (details in the table below)

| #    | Script                    | What it checks                                                                                                                    | Introduction phase                                   | Acceptance                                              |
| ---- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` two-way assertion                                                                              | P4                                                   | Deliberately missing one phase must fail                |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third-file read                                                                | P4 (**only assignment, no longer duplicated in P9**) | Deliberately leaving a consumed-less field must fail    |
| 9.3  | `check-boundary.py`       | Prohibit `include!`; `pub(crate)` cross-layer leakage only decreases; prohibit L2→L3 (**only assignment**, defined in P0 of `08`) | **P0**                                               | Deliberately adding `include!` must fail                |
| 9.4  | `check-test-wiring.py`    | Has `tests/` directory but parent module lacks `mod tests;` → fail                                                                | P1                                                   | Deliberately creating an orphan directory must fail     |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message has no `snapshot-update` marker                                                              | P2                                                   | Same as above                                           |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                              | P2                                                   | Same as above                                           |
| 9.7  | `check-corpus-parity.py`  | Corpus diff non-empty (diagnostic code+span compared item by item, **no grouping relaxation of any kind**)                        | P2                                                   | Same as above                                           |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction appears outside `synth.rs`                                                                               | P7                                                   | Same as above                                           |
| 9.9  | `check-concepts.py`       | Prohibition One A/B/C/D (parallel representations, disambiguation aliases, synonym tables)                                        | **P0** (report-only, turned hard in P9)              | Must report 3 sets of operator enums on unmodified code |
| 9.10 | `check-fanout.py`         | Prohibition Three A/B/C                                                                                                           | **P0**                                               | Deliberately adding a 6th entry-style wiring must fail  |

> **No line-count ratchet script** (`check-file-size.py` + `baseline.toml` was cancelled on
> 2026-10-03). The size problem is solved by `08` Prohibition Two criterion A (responsibility
> separation, manual judgment).

### P10 Remaining Cleanup and Status Corrections

Source: [06](06-cleanup-inventory.md) §Implementation Notes

- [ ] **S2 Delete pure-placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty-head design documents** (1 level-3 task)
- [ ] **S4 RFC status correction** (1 level-3 task)
- [ ] **S5 `pub` items reduce visibility** (1 level-3 task)
- [ ] **S6 opcode dead-path cleanup** (1 level-3 task)

| Level 2                              | Level 3                                                                                                                                                                                                                                                                                                                                                                      | Acceptance                                                                                                                                               |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S2 Delete pure-placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladder / old-syntax detection / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: `lib.rs:568` / `:582` two `_ => todo!()` sites changed to meaningful fallback output**                                                                                                                           | `cargo test` passes; `clippy -D warnings` no new warnings                                                                                                |
| S3 Empty-head design documents       | After C1+C2 are deleted, rewrite to point to `src/frontend/module/` (~2,670 lines); C3 rewrite the boundary table; **simultaneously clean up `config.js:259-268`**                                                                                                                                                                                                           | No 404 on doc site; `check-docs-truth.py` passes                                                                                                         |
| S4 RFC status correction             | C4 RFC-018 moved back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 `TRACKING.md` add "Implementation Status" column (D36)                                                                                                                                                                                                                              | `check_tracking.py` exit code 0 (**don't manually edit TRACKING.md**)                                                                                    |
| S5 `pub` items reduce visibility     | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be kept**)                                                                                                                                                                                                                                                                                                | Compiler can point out the real dead code                                                                                                                |
| S6 opcode dead-path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete, opcode value reclamation already landed in 7e.1 along with version bump); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes the `op` field); **F7+F8 opcode generation-phase gate** (extend `tools/code-tables` opcode extractor ~30 lines + 5 sites of fact-expression single-sourcing + `size()` full-opcode compare test) | 5 sites of opcode fact expression converge to 1 authoritative source; `size()` comment inconsistent with actual encoding (`bytecode.rs:2181-2182`) fixed |

## Implementation Notes: Dependencies and Parallelism

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 hemostasis channel (adjudicated 2026-10-05)**: P3's prerequisite is narrowed to **2.3.2 +
> 2.4.1 + 2.4.3** (multi-file corpus layer + two red oracles). The rest of P2 (2.1 IR verifier, 2.2
> snapshots, 2.3.1/2.3.3 diffing and performance baseline) can proceed in parallel with P3. The
> hemostasis of correctness bugs is not blocked by oracle infrastructure; but **P4 must still wait
> for all of P2 to complete** — the unified Driver needs all three layers of oracles in place.

| Constraint         | Reason                                                                                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1            | The three prohibitions constrain every action in P1 (1.2.x is exactly adding `mod` declarations to `mod.rs`)                                             |
| P1 → P2            | Reviving tests changes test counts and the corpus baseline                                                                                               |
| P2 (narrowed) → P3 | Vulnerability oracles must **be written red first** (2.4.1/2.4.3) + the multi-file corpus layer (2.3.2) must exist, or the red tests have nowhere to run |
| P3 → P4            | Fix bugs first, then refactor; the reverse order lets the bug be固化'd as "established behavior" by the phase table                                      |
| P4 → P5            | The `include!` refactoring and the checker split touch the same file                                                                                     |
| P5 → P6            | Sequential changes to the same file must be split, otherwise regressions can't be bisected                                                               |
| P6 → P7 / P8       | Without first converging type representation, the new IR will grow into a third representation                                                           |
| P7/P8 → P9         | 9.5's snapshot baseline and 9.3's `pub(crate)` leakage count only take their final form after P7/P8                                                      |

### Parallelizable Groups

| Group                                      | Members                                                                            | Basis                                                                                                                                                                                                                                               |
| ------------------------------------------ | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A (the only one authorized by the RFC)** | P7 ∥ P8                                                                            | File sets disjoint: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                                       |
| B                                          | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                      | Each independently adds `mod`, revert file by file                                                                                                                                                                                                  |
| C                                          | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                    | Three layers of oracles independent of each other                                                                                                                                                                                                   |
| D                                          | 4.4 (proof layer wrap-up) ∥ entire P6                                              | `02` explicitly states S5 "should walk independently of S1-S4"                                                                                                                                                                                      |
| E                                          | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                              | All after 4.1 completes, mutually independent                                                                                                                                                                                                       |
| F                                          | 9.3 ∥ 9.9 ∥ 9.10 (the three scripts introduced in P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                                                                                                                                                                          |
| G                                          | S3 ∥ S4 ∥ S5                                                                       | File sets disjoint (verified 2026-10-05: S3 touches `docs/src/dev/design/check/` + `config.js`; S4 touches `docs/src/rfc/` + `check_tracking.py` generator; S5 touches two `pub` sites in `src/frontend/core/typecheck/`; pairwise no intersection) |

**Not parallelizable (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5
same-batch files); 4.3.2 ∥ 9.2 (already unified in P4).

**Constraints added by this document**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial; 7a→7b→7c→7d
strictly serial, and the `Phi` arm in 7b must precede 7d.

## Key Decisions and Rationale

- **P0 stands as its own phase rather than being merged into P9** — gates are "post-hoc checks",
  rules are "pre-hoc judgment". P9's scripts cannot prevent "should have refactored but applied a
  patch"; only D0–D4's review checklist can.
- **P9 as the sole authoritative list of CI scripts** — the three coexisting lists in original gap
  G3 made it impossible to converge. After this table merges them, 10 scripts each belong to one
  phase, with report-only and turn-hard timings explicitly traceable.
- **P5's steps are filled in by this document rather than merged into P4** — P4 is already the
  largest single-point risk; adding checker splitting on top would exceed the rollback granularity.
- **5.3.2 takes "zero caller-side changes" as its hard acceptance** — the split must be
  independently revertible.

## Known Limitations and Risks

- **1.3.x (fix exposed defects) has no regression oracle** — it only says "fixed defects must not be
  rolled back", lacking a recording requirement for "what was fixed". **Execution agreement**: every
  1.3.x fix's commit message must reference the fixed test name and defect description, serving as a
  post-hoc bisection basis.
- ~~**7b.1 too coarse-grained**~~ **Fixed (2026-10-05)**: split into 7b.1a / 7b.1b as two exclusive
  commits per DoD 6.
- **8.2.1 too coarse-grained** — ~89 renamed references; missing one **silently changes
  associativity**.
- ~~**D17 / D20 had no corresponding level-3 tasks**~~ **Fixed (2026-10-05)**: D17 landed as 7e.1,
  D20 landed as 4.5.1/4.5.2; D32/D34's opcode reclamation merged into 7e.1, deletion action left for
  P10 S6.
- ~~**`2315` / `2341` data loss defects originally planned as "independent issue"**~~ **Fixed
  (2026-10-05)**: incorporated as 7e.2 / 7e.3. The exception table (throw/try) is core language
  semantics and must not be left as a legacy.
- ~~**E2 (`lib.rs:568/582` `todo!()`) had no phase attribution**~~ **Fixed (2026-10-05)**: merged
  into P10 S2.
- ~~**F7 (opcode gate extractor) attribution was vague**~~ **Fixed (2026-10-05)**: merged into the
  generation-phase gate task in P10 S6.

## Conflict Log: All Adjudicated

> **The original "Conflict Log" C1–C8 and each document's open issues have all converged into the
> [RFC-039 Resolution Log](../../rfc/draft/039-compiler-architecture.md) (D1–D50).** This document
> leaves no pending items. Item-by-item correspondence:

| Original # | Topic                                                          | Resolution                                                                                                                                                 |
| ---------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1         | `precedence_inline.rs` revived in P1, deleted in P8            | **Revive** (P1's premise is reviving tests with real assertions); P8's 8.6 deletes this file along with Pratt                                              |
| C2         | "Grammar-driven" commitment vs. table-based                    | **Fully done**: adopt LALRPOP, see `05` "Syntax: Full Grammar-Driven"                                                                                      |
| C3         | C6 category definition conflicts with S1 classification        | **S1 falls under C6** (revival does not require equivalence oracle, only regression tests); C6's definition is extended to "pure deletion or pure revival" |
| C4         | `test_release_plan_spans_consumed` whitelist                   | **No whitelist**, difference set must be empty; if it can't be empty, the `ReleasePlan` contract is defective → change to `PlanId` (D20/D41)               |
| C5         | `Aggregation`'s two functions' internal differences unverified | **P4's 4.2.7 has listed it as a required verification item**; verify line by line during implementation                                                    |
| C6         | wasm playground fix needs P4's `ProgramKind`                   | **wasm path's bug fixes merged into 4.2.8**; P3 only handles the three native entries                                                                      |
| C7         | `method_def_ordinals` attribution                              | **Goes to 04** (P7 batch c), see D21                                                                                                                       |
| C8         | `upvalue_count` fix and `.42` version number                   | **Fix + `VERSION` bumped from 4 to 5** (the format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`), see D17                    |

## See Also

- [RFC-039 Body](../../rfc/draft/039-compiler-architecture.md) — the sole authority for phase
  sequence, DoD, and global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 criterion category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0's detailed design and D0–D4
  decision procedure
- [01-routing.md](01-routing.md) — target directory structure (the form after tasks complete)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — design basis for each phase
