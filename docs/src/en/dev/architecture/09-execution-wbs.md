# Multi-level Work Breakdown Structure (WBS)

> **Subsidiary design document**. This document is a subsidiary to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). It
> expands the RFC's phase sequence into **independently committable, independently verifiable**
> three-level tasks.
>
> The phase ordering and acceptance gates are governed exclusively by the body of RFC-039; this
> document is responsible only for decomposition to an executable granularity and for registering
> any unresolved conflicts.

## Positioning and Scope

| Level       | Count       | Meaning                                                                           |
| ----------- | ----------- | --------------------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Phases, one-to-one correspondence with RFC-039's phase sequence                   |
| **Level 2** | 45          | Task groups, drawn from the "Implementation Points" sections of various documents |
| **Level 3** | 122         | Independently committable actions                                                 |

**This document covers**: task decomposition, dependencies, parallel groupings, and acceptance
criterion bindings.

**This document does not cover**: the design rationale for each task (see `02`–`08`), or the
justification for phase ordering (see RFC-039).

## Current State: Gaps Discovered During Decomposition

While consolidating the "Implementation Points" from the 8 documents into this table, **the
documentation system itself was found to have 5 hard gaps**. All have been reconciled:

| #   | Gap                                                                                                                                                                                                                                                                                                                                                                         | Resolution                                                                                                                                                                |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no construction steps**. The RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents describes how to perform this split; the directory tree in `01-routing.md` once mentioned an `annotations.rs` line, which is the only reference in the entire batch                                                                         | P5 steps are filled in by this document (see below)                                                                                                                       |
| G2  | **`include!` refactoring has three attributions, zero landings**. The RFC places it in P4; the cross-reference in `02` assigns it to itself; S7 in `06` also lists it. But the change list in `02` (6 additions + 20 modifications) **does not include `semantic_tokens.rs`**                                                                                               | Construction steps assigned to §P5 5.1 of this document (after P4 is complete); the references in the RFC and 06 have been synchronized to point here                     |
| G3  | **Three parallel CI script lists**. The `RFC` lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` separately lists 4 checks (with completely different names); `04` additionally requires `check-synth-boundary.py`. After deduplication **at least 9 new scripts, with no unified list**; `check-obligations.py` is double-attributed to both P4 and P9 | Unified list in §P9 of this document (10 scripts, each belonging to one phase); all other documents reference this table                                                  |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it among the "1005 lines to be resurrected", while B6 in the same document says it and the `Precedence` enum "must be deleted together"; `05` argues for deletion                                                                                                                                           | **Resurrect in P1, delete in P8 along with Pratt** (resolution registered as C1)—first restore CI coverage; the deletion occurs in P8's 8.6 together with the dead ladder |
| G5  | **L2 goal statement conflict**. The RFC and this directory's index write "grammar-driven syntax", but `05` self-evaluates as "**did not adopt true grammar-driven (LALRPOP), only achieved declarative table lookup**"                                                                                                                                                      | Reconciled to **LALRPOP fully grammar-driven** (05 §Syntax: Full Grammar-Driven); G7/G8 acceptance gates execute per this commitment                                      |

## Goal Design: Phase Sequence

```
P0  Repository Maintenance Mechanism and Code Placement Rules  08        ← New: establish rules first
P1  Resurrect Orphan Tests and Fix Defects                       06 §S1
P2  Establish Equivalence Criterion Baseline                     07 all
P3  Fix Correctness Vulnerabilities (Minimal Plan)               02 §S3-Vulnerability Half
P4  Stage Contract and Unified Driver                            02
P5  checker File Internal Split                                  02 + supplemented by this doc
P6  Unify Type Representation                                    03
P7  Intermediate Representation SSA-ification                    04      } Runnable in parallel
P8  Frontend Paradigm Change                                     05      }
P9  Anti-regression Gates (including Unified Script List)        01 §Anti-regression + 08 §Machine-checkable Rules
P10 Other Cleanup and Status Corrections                         06 §S2/S3/S4/S5/S6
```

**Reason for the new P0**: The three prohibitions (no fabrication / no endless filling-in / patch
instead of refactor) constrain every action in P1–P10. The rules must be established before work
begins; otherwise, when P1 resurrects tests, one is already adding `mod` declarations to
`tests/mod.rs`—which is precisely the cause of conflict G4.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [x] **0.1 Document the rules** (3 level-3 tasks; 0.1.2/0.1.3 landed early on 2026-10-05; 0.1.1
      passed review on 2026-10-06: content verified + full Chinese localization of CONTRIBUTING.md,
      English version migrated to docs/gh/CONTRIBUTING.en.md)
- [x] **0.2 Gate implementation** (3 level-3 tasks, completed 2026-10-06: check-concepts /
      check-fanout / check-boundary all wired into CI as report-only (concepts job), all negative
      probes pass (new entry wiring red / L2→L3 reverse use red / 3 sets of operator enums and 2
      sets of parallel type representations including `ir::Type` alias detected on unchanged code),
      `pub(crate)` baseline 137, P9 unified switch to hard)
- [ ] **0.3 Table gate promotion** (1 level-3 task)

| Level 2                  | Level 3                                                                                                                                                                                                                                                                                                                                                             | Prerequisite | Acceptance                                                                                                                                                                             |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Document the rules   | 0.1.1 Write D0–D4 + three prohibitions into `CONTRIBUTING.md`; responsibility categories reference the directory responsibility table in `01-routing.md`                                                                                                                                                                                                            | —            | Human review passed                                                                                                                                                                    |
|                          | 0.1.2 Implementer touchpoints land (**completed 2026-10-05**): create `AGENTS.md` (root directory entry) + `compiler-architecture/HOWTO.md` (pre-work self-check + D3 patch determination); add a "Responsibility Attribution and Decision Procedure" required block to the PR template (D44); add a "Code Placement and Change Rules" section to `CONTRIBUTING.md` | —            | Landed; review verifies the four touchpoints                                                                                                                                           |
|                          | 0.1.3 Extract the rule body (**completed 2026-10-05**): create `docs/src/dev/coding-rules.md` (precise rule description, long-term validity, not archived with RFC-039); 08 adds a migration header note converted to a diagnostic record; all four touchpoints redirected to coding-rules                                                                          | —            | Landed                                                                                                                                                                                 |
| 0.2 Gate implementation  | 0.2.1 `check-concepts.py` (Prohibition 1 A/B/C/D)                                                                                                                                                                                                                                                                                                                   | 0.1.1        | **Must report 3 sets of operator enums and 2 sets of parallel type representations (including `ir::Type` alias) on unchanged code**; report-only at this stage, switched to hard at P9 |
|                          | 0.2.2 `check-fanout.py` (Prohibition 3 A/B/C)                                                                                                                                                                                                                                                                                                                       | 0.1.1        | **Deliberately adding a 6th entry-style wiring must turn red**; report-only at this stage, switched to hard at P9                                                                      |
|                          | 0.2.3 `check-boundary.py` (Prohibition 2 Criterion C)                                                                                                                                                                                                                                                                                                               | 0.1.1        | **Deliberately adding one L2→L3 reverse `use` must turn red**; record the `pub(crate)` leak count as the initial value                                                                 |
| 0.3 Table gate promotion | 0.3.1 Extend `tools/code-tables` to cover opcode / type tables                                                                                                                                                                                                                                                                                                      | 0.1.1        | Depends on P6's T1; closed out independently                                                                                                                                           |

> **No scale baseline task**. The 2026-10-03 decision cancels all line-count / size gates; the scale
> problem is resolved by responsibility separation (see `08` Prohibition 2 Criterion A, human
> judgment). **0.2.1 is the core acceptance for this stage**: if the gate cannot report the known
> parallel representations, the criterion design is wrong—it's a fake gate.

### P1 Resurrect Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Points S1 + [05](05-frontend-paradigm.md)
§Test Reconstruction

- [x] **1.1 Resurrect `lexer/tests/`** (4 level-3 tasks, completed 2026-10-06: 55 lexer tests
      online; delete 7 empty shells; merge `#[path]` bypass into the main wiring to avoid duplicate
      runs)
- [x] **1.2 Resurrect 4 locations in live directories** (completed 2026-10-06: precedence_inline 6 +
      overload_inline 7 + json 3 + template 7; additionally delete 5 lines of
      `typecheck/tests/semantic_db.rs` remnants)
- [x] **1.3 Fix exposed defects** (completed 2026-10-06, **actual results differ from predictions**:
      the overflow path and `\x`/`\u` escape sequences are currently safe in the code; 20 probe
      tests fill coverage that never existed; the real defect exposed is "illegal alphanumeric
      adjacency after a base-prefixed literal is silently split"—`0b102` compiles and runs producing
      the wrong value 2, `0o128` produces 10, `0x1FG`/`123abc` produce misleading E1001. Fix:
      `literals.rs` adds a shared helper `reject_trailing_alnum` (compliant with Prohibition 3: one
      implementation, five call sites), all 5 red-then-green tests pass)

| Level 2                                       | Level 3                                                                                                                                                 | File:Line                  | Prerequisite | Acceptance                                                                     |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | ------------------------------------------------------------------------------ |
| 1.1 Resurrect `lexer/tests/`                  | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                     | after `lexer/mod.rs:106`   | —            | Test count rises                                                               |
|                                               | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                             | `lexer/tests/mod.rs:15-25` | 1.1.1        | **Without these, the 159 lines still don't run**                               |
|                                               | 1.1.3 Delete 7 empty-shell files + their `mod` declarations                                                                                             | 7 files in `lexer/tests/`  | 1.1.1        | C6 has no criterion                                                            |
|                                               | 1.1.4 Delete `pub use ...::*;` re-export block                                                                                                          | `tests/mod.rs:28-38`       | 1.1.3        | This block is exactly what masked the empty shells                             |
| 1.2 Resurrect 4 locations in live directories | 1.2.1–1.2.4 Wire `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` respectively | each parent module         | —            | Test count rises; **1.2.1 conflicts with 8.0.1, see conflict registration C1** |
| 1.3 Fix exposed defects                       | 1.3.1 Four base scanners' overflow paths                                                                                                                | `literals.rs`              | 1.1.x        | `cargo test` green                                                             |
|                                               | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                    | `literals.rs`              | 1.1.x        | Same as above                                                                  |

**Acceptance**: Test count rises; **real defects are expected to be exposed**, so reserve time for
fixes. Revert file by file, but **the fixed defects should not be rolled back**.

### P2 Establish Equivalence Criterion Baseline

Source: [07](07-equivalence-oracle.md) all (this document has no "Implementation Points" section;
derived inversely from its structure)

- [x] **2.1 IR static verifier** (3 level-3 tasks, all completed 2026-10-06: verify.rs dual mode + 7
      invariants + corpus green, actually fixed 3 ir_gen defects)
- [x] **2.2 Normalized snapshots** (2 level-3 tasks, completed 2026-10-07: normalize.rs + 204
      snapshots archived + gate 9.5 wired into CI)
- [ ] **2.3 Corpus differential** (3 level-3 tasks)
- [x] **2.4 Vulnerability-specific criteria** (3 level-3 tasks, all completed 2026-10-06:
      2.4.1/2.4.3 turned green with P3, 2.4.4 red state staged for P4/D20)
- [ ] **2.5 Regression gates** (1 level-3 task)

| Level 2                             | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Prerequisite | Acceptance                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------ | --------------------------------------------------------------------------------------- |
| 2.1 IR static verifier              | 2.1.1 `verify.rs` dual mode `verify_loose` / `verify_ssa` (**completed 2026-10-06**: `src/middle/core/verify.rs`, 07 §102 contract form `verify(ir, mode) -> Result<(), VerifyError>`, violations collected in full rather than failing on first; def/use extraction uses exhaustive match with no wildcard arm, so introducing a Phi variant causes compilation failure forcing a check update)                                                                                                                                                                                                                                           | —            | C4                                                                                      |
|                                     | 2.1.2 **`verify_loose` runs green on existing non-SSA IR** (**completed 2026-10-06**: `test_verify_loose_corpus_green` runs green on 204 files / 536 functions / 11990 instructions across the full corpus, with built-in non-empty verification surface assertions; actually fixed 3 ir_gen defects per D38, corpus differential baseline self-comparison proves zero differential = behavior equivalence)                                                                                                                                                                                                                                | 2.1.1        | **Hard threshold, entry condition for P7 batch a**—**unlocked**                         |
|                                     | 2.1.3 Implement 7 invariants (dominance / unique definition / Phi consistency / jump targets / out-of-bounds globals / type consistency / inner isolation) (**completed 2026-10-06**: judgment surface narrowed by actual measurement—dominance = named slots must-defined (temp slots with Void pre-initialization are frame semantics intentionally relied upon by ir_gen), type consistency = parameter slot signatures + constant trusted sources (ir_gen does not write the slot type table, the complete surface belongs to P7 value table 04 §262); Phi pending batch b variant introduction; see verify.rs file header and 06 §G6) | 2.1.1        | C4                                                                                      |
| 2.2 Normalized snapshots            | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / sort predecessors) (**completed 2026-10-07**: `src/middle/core/normalize.rs`, five rules from 07 implemented item by item; two corrections made—DefId relativized by first-occurrence order (intern order is unstable across processes, measured at 141/143), Arg uses independent arg% prefix (sharing a pool with Local loses variant distinction); instruction printing uses exhaustive match)                                                                                                                                                           | —            | C1/C3/C5                                                                                |
|                                     | 2.2.2 Snapshots archived + human review process (`src/middle/core/tests/snapshots/`) (**completed 2026-10-07**: 204 snapshots archived mirroring corpus relative paths; update process `UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored` + git diff review + same PR commit, see tests/snapshot.rs file header; tampering measured to be precisely reported as red)                                                                                                                                                                                                                                                              | 2.2.1        | C1/C3/C5                                                                                |
| 2.3 Corpus differential             | 2.3.1 Differential framework + 293 corpus baseline (**completed 2026-10-06**: probe `examples/corpus_probe.rs` + baseline `tests/baselines/corpus-parity.jsonl` (322 entries) + gate `check-corpus-parity.py` wired into CI (hard gate); normalization includes pointer-form scrub, two full runs of the gate self-compares to zero differential)                                                                                                                                                                                                                                                                                          | —            | Per item: diagnostics / exit codes / stdout                                             |
|                                     | 2.3.2 **Multi-file corpus layer** (new `tests/yaoxiang-multifile/`, with `yaoxiang.toml` project fixtures—resolution D48) (**completed 2026-10-06**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | —            | **Unconditional must-do (D40), the sole source of executable behavior criteria for P4** |
|                                     | 2.3.3 C4 behavior differential mandatory coverage list (10 semantic categories) + performance baseline (criterion smoke benchmarks)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                                  |
| 2.4 Vulnerability-specific criteria | 2.4.1 `test_multifile_proof_obligation_not_dropped` (**completed 2026-10-06: red state staged**, actual measurement single-file ["E4018"] vs compile_project [])                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 2.3.2        | **Must first be red**                                                                   |
|                                     | 2.4.3 `test_no_silent_pass_on_unproven` (**completed 2026-10-06: red state staged**, trigger source measured to be silent under compile_project)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | —            | Blocks `checker.rs:5179/5318/5448`                                                      |
|                                     | 2.4.4 `test_release_plan_spans_consumed` (**completed 2026-10-06: red state staged**, measured 168 files / 373 keys unconsumed—D20 contract defect evidence, contract fix belongs to P4/D20)                                                                                                                                                                                                                                                                                                                                                                                                                                               | —            | **No whitelist, the difference set must be empty (D41)**                                |
| 2.5 Regression gates                | 2.5.1–2.5.4 see the P9 unified list                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 2.1–2.3      | See P9                                                                                  |

> **P2 landing (2026-10-06, 2.1)**: verify.rs dual mode + 7 invariants + full corpus green. **D38
> prophecy fulfilled**—the green-run process actually discovered and fixed 3 real defects in ir_gen
> (see 06 §G6 for details): method locals table truncated to parameter count (local_count
> underestimated, runtime relies on `Frame::set_slot` for tolerance), unannotated parameters dropped
> by `filter_map` (three locations, params metadata misaligned with frame layout), match result slot
> relied on frame's implicit Void initialization (now explicitly pre-initialized). The actual
> semantics of jump targets is flattened instruction indices (not label-keyed as in the 07 text),
> and the CFG is built at instruction level—see verify.rs file header. **Commit correspondence**:
> ir_gen fix = `a9ea199b`; verify body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision
> caused its message to be mislabeled as "docs(rfc): RFC-027a…", but the content is actually
> verify.rs + 6 test files; buried in history, unamendable, registered here for the record).
>
> **P2 narrowed partial landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all staged, **P3 stop-bleeding
> channel preconditions unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (4 behavior + 1 compile-error) + runner
>   `tests/yx_multifile_runner.rs`; the judgment contract uses the same `TestFileSpec` as the
>   single-file corpus (single source of truth on the library side), binary location / subprocess
>   spawning extracted to `tests/common/` and yx_runner migrated in sync (Prohibition 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. The trigger source uses the
>   `SumUpTo(3, r)` form nailed down by the rfc027 tests (return position / call site / binding
>   position)—the `Sorted(3)` annotation example in 02 would be substituted and evaluated directly
>   to Disproved under current code, not presenting silent Unproven; the `Sorted(3)` in 07
>   pseudocode is illustrative only. Red-state measurement data is in the test file header.
> - The two red criteria are mounted with `#[ignore]` for permanent residence following the 2.4.5
>   red skeleton precedent: default suite all green, `-- --ignored` reproduces the red state;
>   **remove the attribute when P3 fix lands to turn green**.
> - The rationale for the deliberate deviation from the 07 (code, file, line) table in the
>   comparison normalization (removing file/line, comparing only error code sets) is written in the
>   2.4.1 test comment.
>
> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **are moved out of P2**: the `Program` / `Obligations` types they reference don't exist until P4,
> and P2 cannot compile them. The former becomes P4's 4.1.4 (don't double-register), and the
> latter's `#[ignore]` red skeleton is merged into 4.3.1.

### P3 Fix Correctness Vulnerabilities (Minimal Plan)

Source: [02](02-stage-contract.md) §Implementation Points S3 vulnerability fix half

- [x] **3.1 `proof_calls` consumer side** (2 level-3 tasks, completed 2026-10-06)
- [x] **3.2 Second silent drop point** (1 level-3 task, completed 2026-10-06)
- [x] **3.3 panic → diagnostic** (1 level-3 task, completed 2026-10-06)

| Level 2                         | Level 3                                               | File:Line                                | Prerequisite | Acceptance                          |
| ------------------------------- | ----------------------------------------------------- | ---------------------------------------- | ------------ | ----------------------------------- |
| 3.1 `proof_calls` consumer side | 3.1.1 Make `proof_calls` private + single getter      | `types.rs:29`                            | 2.4.1        | Vulnerability criterion turns green |
|                                 | 3.1.2 Complete the 3 consumer sites                   | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1        | C2                                  |
| 3.2 Second silent drop point    | 3.2.1 `Unproven` empty match arm produces diagnostic  | `checker.rs:1303-1314`                   | 2.4.3        | `test_no_silent_pass_on_unproven`   |
| 3.3 panic → diagnostic          | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic | `predicate.rs:34-36`                     | —            | No longer panics when Z3 is missing |

**Acceptance**: Vulnerability criteria turn green, and **deliberately reverting the fix must turn
them red again**. **Risk**: Adding E4018 to the multi-file path is a breaking change.

> **P3 landing (2026-10-06)**: 3.1.1 `proof_calls` becomes `pub(crate)` + single getter (fully
> private would require rewriting 11 literal constructions, not taken in the stop-bleeding phase;
> closed out at P4 Obligations migration); execution mechanism extracted to
> `src/frontend/proof_execution.rs` (Prohibition 3 single point—cannot go in typecheck/, that would
> create L3→L4 reverse dependency); 3.1.2 completes **four entry points** per 02 evidence chain step
> 5 (WBS missed line number `compile_project`:99, must be completed and registered per 3.1.1's
> "criterion turns green" acceptance); 3.2.1 empty arm changed to bookkeeping (throw on
> proof_calls) + diagnostic (the established `into_result` conversion path); 3.3.1 SOLVER slot
> Option-ified, Z3 missing gracefully degrades to `SMTResult::Unknown` (E2031 family diagnostic
> replaces process crash). Both vulnerability criteria have `#[ignore]` removed and turn normal
> green; corpus fixture `proof-obligation-honored` added; check-fanout inventory migrated
> accordingly (wiring point 19 = inventory). Measured: criteria 2.4.1/2.4.3 turn green, full suite
> lib 2494 / integration 272 / both corpus layers all green.
>
> **Stop-bleeding channel (2026-10-05 ruling)**: P3 does not need to wait for all of P2. The only
> hard prerequisites for 3.1/3.2 are **2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red
> criteria)**; 2.1/2.2/2.3.1/2.3.3 (IR verifier, snapshots, single-file differential, performance
> baseline) can proceed in parallel with P3. Stop-bleeding for correctness vulnerabilities
> (`Sorted(3)` silent pass) should not be blocked by snapshot infrastructure construction. 3.3
> (panic → diagnostic) has no prerequisites and can land at any time.

### P4 Stage Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Points S1–S5. **This phase and P3's fixes must be
in two separate commits.**

- [ ] **4.1 Declarative stage table** (5 level-3 tasks)
- [ ] **4.2 Entry consolidation** (8 level-3 tasks)
- [ ] **4.3 Obligation ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm wrap-up** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ification (D20)** (2 level-3 tasks)
- [ ] **4.6 LSP semantic data pipeline unification (D55)** (1 level-3 task)

| Level 2                                         | Level 3                                                                                                                                                                       | Prerequisite | Acceptance                                                                                                                                                     |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative stage table                     | 4.1.1 `stage.rs` (`Stage` variant + `Stage::ALL` + `StageScope`)                                                                                                              | P0           | `test_program_stage_coverage`                                                                                                                                  |
|                                                 | 4.1.2 `program.rs` / `unit.rs`                                                                                                                                                | 4.1.1        | Same as above                                                                                                                                                  |
|                                                 | 4.1.3 `Pipeline::run` reroutes through Driver (`pipeline.rs:141-227`)                                                                                                         | 4.1.2        | **Single-file diagnostic set and exit code byte-identical**                                                                                                    |
|                                                 | 4.1.4 Stage coverage assertion                                                                                                                                                | 4.1.3        | C2                                                                                                                                                             |
|                                                 | 4.1.5 Full corpus zero-diff                                                                                                                                                   | 4.1.4        | Diagnostics / exit code / stdout all zero-diff (293 corpus)                                                                                                    |
| 4.2 Entry consolidation                         | 4.2.1 Slim `compile_project` to a Program constructor (`orchestrator.rs:99-237`)                                                                                              | 4.1.5        | Four entry points' diagnostic sets identical after unification                                                                                                 |
|                                                 | 4.2.2 `check_project` (`:273-399`)                                                                                                                                            | 4.2.1        | Same as above                                                                                                                                                  |
|                                                 | 4.2.3 `check_source_in_project` (`:450`)                                                                                                                                      | 4.2.2        | Same as above                                                                                                                                                  |
|                                                 | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                                                                     | 4.2.3        | Same as above                                                                                                                                                  |
|                                                 | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                                                                | 4.2.1        | `check` consistent inside and outside the project                                                                                                              |
|                                                 | 4.2.6 Delete LSP manual stage sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                                                                | 4.2.1        | LSP and CLI diagnostic sets identical                                                                                                                          |
|                                                 | 4.2.7 `Aggregation` parameter-driven binary choice (`orchestrator.rs:486-494`)                                                                                                | 4.2.1        | **Verify line-by-line the internal differences between the two functions during implementation** (conflict registration C5 ruled to require this verification) |
|                                                 | 4.2.8 wasm reroutes through `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                                                                     | 4.2.1        | See C6                                                                                                                                                         |
| 4.3 Obligation ledger                           | 4.3.1 `obligations.rs` + `assert_drained()` (new; includes `#[ignore]` red skeleton first)                                                                                    | 4.1.3        | `test_obligations_drained` turns green                                                                                                                         |
|                                                 | 4.3.2 Obligation diagnostic W → E upgrade                                                                                                                                     | 4.3.1        | Human review every new E                                                                                                                                       |
| 4.4 Proof layer and wasm wrap-up                | 4.4.1 Update `layers/README.md` layer ordering to actual order                                                                                                                | —            | **Proceeds independently of 4.1–4.3**                                                                                                                          |
|                                                 | 4.4.2 `default_solver()` becomes `&'static` singleton (`proof/smt/backend.rs:67-72`)                                                                                          | —            | Cache hit rate observable                                                                                                                                      |
|                                                 | 4.4.3 Add warning to degradation path (`checker.rs:1283-1293`)                                                                                                                | —            | —                                                                                                                                                              |
| 4.5 Cross-layer contract PlanId-ification (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (allocation on producer side at `layers/ownership.rs:31` + consumer side match at `ir_gen.rs:1942`)                                 | 4.3.1        | `test_release_plan_spans_consumed` difference set empty (D41)                                                                                                  |
|                                                 | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                                                            | 4.5.1        | C2; span mismatch class of silent failures drops to zero                                                                                                       |
| 4.6 LSP semantic data pipeline unification      | 4.6.1 Project-internal files' semantic data (SemanticDB) goes through the orchestrator's same pipeline, same source as diagnostics; cross-file references carry `resolves_to` | 4.2.1        | Cross-file jumps within the project hit definitions; semantic data and diagnostics come from the same compilation                                              |

### P5 checker File Internal Split — supplemented by this document (original gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (the `pub(crate)` path of
`collect_used_in_type` must not change)

- [ ] **5.1 `include!` → real `mod`** (2 level-3 tasks)
- [ ] **5.2 Extract `refinement`** (2 level-3 tasks)
- [ ] **5.3 Extract `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                                          | Prerequisite | Acceptance                                                        |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------------------- |
| 5.1 `include!` → real `mod` | 5.1.1 Change the `include!` at `checker.rs:5618` to a real `mod` declaration                                                                                                                                                                                     | P4 complete  | C1 zero-diff; `semantic_tokens.rs` gains module identity          |
|                             | 5.1.2 Add `use super::*` or item-by-item imports (`include!` era implicitly inherited scope)                                                                                                                                                                     | 5.1.1        | `cargo build` green                                               |
| 5.2 Extract `refinement`    | 5.2.1 Migrate out the refinement block (currently `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                           | 5.1.1        | C1 zero-diff                                                      |
|                             | 5.2.2 Refactor `collect_refined_binding_checks` signature: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                           | 5.2.1        | C1; only call site is `1235`                                      |
| 5.3 Extract `annotations`   | 5.3.1 Migrate out annotation validation (currently `3441-3661`, including `is_predicate_head`)                                                                                                                                                                   | 5.2.2        | C1 zero-diff                                                      |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) remains `pub(crate)` and stays under the `checker` module path** → add `pub(crate) use annotations::*;` at the top of `checker.rs`, so that the import at `inference/statements.rs:16` stays **unchanged** | 5.3.1        | **Zero caller-side changes** is the hard acceptance for this step |

**Hard constraint**: `collect_used_in_type` is used cross-module by `inference/statements.rs:16`.
The approach is `pub(crate) use` re-export, making the split a **pure migration**.

**Only split confirmed large blocks.** Don't split modules that are only 200–400 lines—it increases
navigation cost, reduces cohesion, and provides zero decoupling benefit. Other submodules
(signatures / type_defs / imports) will wait until the file grows back to 1,500 lines.

### P6 Unify Type Representation

Source: [03](03-type-unification.md) §Implementation Points Phases 0–6, criterion C3. **Phases 0→6
strictly serial.**

- [ ] **6.0 Gate first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead variant deletion** (2 level-3 tasks)
- [ ] **6.4 parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variants** (2 level-3 tasks)
- [ ] **6.6 Gate to hard** (1 level-3 task)
- [ ] **6.7 Directory rename (D1, pure migration batch)** (1 level-3 task)
- [ ] **6.8 Annotation checking mode and Fn parameter representation (D54, added 2026-10-05)** (2
      level-3 tasks)

| Level 2                                                            | Level 3                                                                                                                                                                                                                                                                                                                                                                                            | File:Line                                                                     | Acceptance                                                                                                                                                                                                                           |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 6.0 Gate first                                                     | 6.0.1 New `tools/type-tables`, implement T1/T2/T3, **T1 initially reports only, does not fail**                                                                                                                                                                                                                                                                                                    | new crate                                                                     | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge reconstruction points listed separately**                                                                                                               |
|                                                                    | 6.0.2 Take full baseline on unchanged code                                                                                                                                                                                                                                                                                                                                                         | —                                                                             | No code changes                                                                                                                                                                                                                      |
| 6.1 Name normalization                                             | 6.1.1 Count non-canonical type name occurrences in the corpus                                                                                                                                                                                                                                                                                                                                      | `tests/yaoxiang/`                                                             | Decide whether synonyms can be deleted directly                                                                                                                                                                                      |
|                                                                    | 6.1.2 `from_builtin_name` cut synonyms → lexer alias table                                                                                                                                                                                                                                                                                                                                         | `mono.rs:618-643`                                                             | C3                                                                                                                                                                                                                                   |
|                                                                    | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                                                                                                                                                                                                               | `types.rs:162-165`                                                            | **Must verify line-by-line; cannot assume equivalence**                                                                                                                                                                              |
| 6.2 Bytecode type unification                                      | 6.2.1 Three locations change to `MonoType`                                                                                                                                                                                                                                                                                                                                                         | `bytecode.rs:812/814/854`, `image.rs:44`                                      | C3 + `dump_bytecode` targeted compare                                                                                                                                                                                                |
|                                                                    | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                                                                                                                                                                                                           | `bytecode.rs:2352-2390`                                                       | Same as above                                                                                                                                                                                                                        |
|                                                                    | 6.2.3 `type_table` element type changes to `MonoType`                                                                                                                                                                                                                                                                                                                                              | `image.rs:44`                                                                 | Same as above                                                                                                                                                                                                                        |
| 6.3 Dead variant deletion                                          | 6.3.1 Delete 11 variants + clean match arms + fallback                                                                                                                                                                                                                                                                                                                                             | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                                   | C3 + `cargo build` green                                                                                                                                                                                                             |
|                                                                    | 6.3.2 **Must be in the same batch as 6.3.1** rewrite the reverse bridge                                                                                                                                                                                                                                                                                                                            | `passes/mono/function.rs:507-529` (`mono_to_ast_type` reconstructs 7 of them) | Generic substitution targeted compare, otherwise **silently changes behavior**                                                                                                                                                       |
| 6.4 parser data flow                                               | 6.4.1 Add `probe.rs`                                                                                                                                                                                                                                                                                                                                                                               | new                                                                           | C3                                                                                                                                                                                                                                   |
|                                                                    | 6.4.2 Change `parser_state.rs:46-58`                                                                                                                                                                                                                                                                                                                                                               | —                                                                             | Same as above                                                                                                                                                                                                                        |
|                                                                    | 6.4.3 Delete `operator_interfaces::spec()` call (**2 locations**: `declarations.rs:509`, `ast.rs:930`)                                                                                                                                                                                                                                                                                             | —                                                                             | Same as above                                                                                                                                                                                                                        |
|                                                                    | 6.4.4 Merge the two `name_used_as_type*`                                                                                                                                                                                                                                                                                                                                                           | `ast.rs:847` + `declarations.rs:42`                                           | Same as above                                                                                                                                                                                                                        |
|                                                                    | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) wires through `NameKind::Builtin`                                                                                                                                                                                                                                                                                                                         | —                                                                             | Same as above                                                                                                                                                                                                                        |
| 6.5 AST dead variants                                              | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                                                                                                                                                                                                                         | `ast.rs:37-43` + 4 consumer locations + 2 exhaustive arms                     | `tests/integration/` 18 modules green (**split into 2 commits**)                                                                                                                                                                     |
|                                                                    | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                                                                                                                                                                                                                    | `ast.rs:241-249`                                                              | Note **semantic risk**                                                                                                                                                                                                               |
| 6.6 Gate to hard                                                   | 6.6.1 T1 changes from warning to `panic!`                                                                                                                                                                                                                                                                                                                                                          | `tools/type-tables`                                                           | **Deliberately introducing a zero-construction variant must turn red**                                                                                                                                                               |
| 6.7 Directory rename (D1, pure migration batch)                    | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; split `parser/ast.rs` into top-level `ast/`; merge bytecode domain (two `bytecode.rs` → `bytecode/`) + migrate `opcode.rs` to `middle/bytecode/`                                                                                                                                                                                          | whole repo                                                                    | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline does not regress                                                                                                                                           |
| 6.8 Annotation checking mode and Fn parameter representation (D54) | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>` (name and type cohabit), closed out in the same batch as 6.5.2 deleting `Assign.signature_params`—6.5.2 only deletes the old representation, this item provides the replacement                                                                                                                                                                  | `ast.rs:451-454`, ~20 consuming files (mechanical changes)                    | C3 (pure representation reshaping batch, compared against the unused); separate commits                                                                                                                                              |
|                                                                    | 6.8.2 Declaration-driven checking mode: parameter types drive lambda header positionally (independent of name), exit (tail expression and return) uniformly compared against annotated return type, non-lambda values infer+unify (isomorphic to `m: Int = "hello"`); red criteria first—u1–u4/v1–v5 probes added to corpus (red then green); delete `declarations.rs:337-376` parser name merging | `statements.rs:1170-1914`                                                     | The comparison-enabled batch is a **behavior fix**: new diagnostic codes, baseline updates, C3 does not apply; v1–v5 five syntactic forms share the same judgment (eliminate "same semantics, different syntax, one red, one green") |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA-ification

Source: [04](04-ssa.md) §Implementation Points Batches a–d, criterion C4, **strictly serial within a
batch**.

- [ ] **7a Cut multiple definitions** (3 level-3 tasks)
- [ ] **7b SSA form switch** (6 level-3 tasks; 7b.1 has been split into 1a/1b per DoD 6)
- [ ] **7c Make implicit contracts explicit** (3 level-3 tasks)
- [ ] **7d Final** (2 level-3 tasks)
- [ ] **7e `.42` data loss fix** (3 level-3 tasks; 2315/2341 absorbed from "independent issue",
      2026-10-05)

| Level 2                             | Level 3                                                                                                                                       | Prerequisite                        | Acceptance                                         |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------------- |
| 7a Cut multiple definitions         | 7a.1 Delete 3 statement-level reclaims                                                                                                        | **2.1.2 `verify_loose` runs green** | C4                                                 |
|                                     | 7a.2 6 save/restore locations replaced with RAII guard                                                                                        | 7a.1                                | C1 snapshot zero-diff                              |
|                                     | 7a.3 Caliber unification (`1798` vs `2001`)                                                                                                   | 7a.2                                | C1                                                 |
| 7b SSA form switch                  | 7b.1a `ir.rs` structure changes #1-2 (`Operand` variant convergence + `Instruction::Phi` introduction)                                        | 7a.3 + **P6 complete**              | C4; exclusive commit                               |
|                                     | 7b.1b `ir.rs` structure changes #3-5 (`FunctionIR` / `BasicBlock` / type annotation fields)                                                   | 7b.1a                               | C4; exclusive commit                               |
|                                     | 7b.2 `next_temp_reg` changes to `Operand::Value`                                                                                              | 7b.1                                | Same as above                                      |
|                                     | 7b.3 Add `Phi` arm to `translator.rs`                                                                                                         | 7b.1                                | C4 + `.42` round-trip test                         |
|                                     | 7b.4 Linear scan allocator (within u8/255 slot model)                                                                                         | 7b.3                                | Same as above (+500~1000 lines)                    |
| 7c Make implicit contracts explicit | 7c.1 `synth.rs` boundary + boundary check script                                                                                              | 7b.4                                | C4 + new script added to CI                        |
|                                     | 7c.2 span consumption count                                                                                                                   | 7c.1                                | Consumption count **zero triggers**                |
|                                     | 7c.3 `method_def_ordinals` read-only                                                                                                          | 7c.1                                | Attribution unclear (C7)                           |
| 7d Final                            | 7d.1 Split `generate_call_expr_ir` into `CallArgs` + 6 `emit_*`                                                                               | 7c.x                                | C1 snapshot zero-diff                              |
|                                     | 7d.2 "Fill 0" fallback changes to return diagnostic                                                                                           | 7d.1                                | C4                                                 |
| 7e `.42` data loss fix              | 7e.1 `upvalue_count: 0` fix (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + already-allocated unused opcode reclaim (D32/D34 with version bump)  | 7d.x                                | C4 + `.42` round-trip test; exclusive commit       |
|                                     | 7e.2 `exception_handlers` persisted (`bytecode.rs:2315`—throw/try are language core semantics, **must not be left for later implementation**) | 7e.1                                | `.42` direct-run exception case differential       |
|                                     | 7e.3 `globals` persisted (`bytecode.rs:2341`)                                                                                                 | 7e.1                                | `.42` direct-run global variable case differential |

**Expectation management**: Net line count **increases 900–1600 lines**. If "code gets shorter" is
set as the success criterion at project inception, this phase will be judged as a failure.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Points Phases 0–4, criterion C5.

- [ ] **8.0 Dead ladder cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexer convergence** (2 level-3 tasks)
- [ ] **8.3 Build LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual parser differential** (1 level-3 task)
- [ ] **8.5 Switch stream** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 Split `parse_assign_after_target`** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)
- [ ] **8.9 Parameter position form normalization (D53, added 2026-10-05)** (1 level-3 task)

| Level 2                                         | Level 3                                                                                                                                                                                                                                                                                                                                           | Prerequisite                     | Acceptance                                                                                                             |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead ladder cleanup (C6)                    | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                                                                                                                                                                                                                                                                   | **Conflicts with 1.2.1, see C1** | C6 has no criterion                                                                                                    |
|                                                 | 8.0.2 Name bare magic numbers (`(6,7)` / `(11,1)` / `12`)                                                                                                                                                                                                                                                                                         | —                                | Same as above                                                                                                          |
| 8.2 Lexer convergence                           | 8.2.1 Merge four base scanners + merge three escape locations + merge multi-line strings                                                                                                                                                                                                                                                          | —                                | C5; `literals.rs` reduced by ~500 lines                                                                                |
|                                                 | 8.2.2 f-string nested compilation elimination                                                                                                                                                                                                                                                                                                     | 8.2.1                            | **span must change**, baseline must be established first                                                               |
| 8.3 Build LALRPOP grammar                       | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                                                                                                                                                                                                                                                                     | 8.2.x                            | `cargo build` passes; can produce AST for 293 corpus (**no comparison**)                                               |
|                                                 | 8.3.2 Action code `grammar/actions.rs` (22 `Expr` variants each with one function)                                                                                                                                                                                                                                                                | 8.3.1                            | Same as above                                                                                                          |
|                                                 | 8.3.3 Error productions (preserves `Expr::Error` / `StmtKind::Error` placeholder semantics)                                                                                                                                                                                                                                                       | 8.3.2                            | Same as above                                                                                                          |
| 8.4 **Dual parser differential**                | 8.4.1 Both parsers each run 293 corpus + `src/std/tests`, AST normalized and **compared bit-by-bit**                                                                                                                                                                                                                                              | 8.3.3                            | **Valid programs' AST fully equivalent—this is the equivalence proof for the entire grammar migration**                |
| 8.5 Switch stream                               | 8.5.1 `parse()` calls LALRPOP, Pratt kept as `parse_legacy()`                                                                                                                                                                                                                                                                                     | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span identical item-by-item (**no relaxation**, no C5′)              |
| 8.6 Delete Pratt                                | 8.6.1 Delete `nud.rs`(1326) + `led.rs`(451) + two sets of BP ladders + ~89 references                                                                                                                                                                                                                                                             | 8.5.1                            | Corpus all green; `git grep BP_` zero hits                                                                             |
|                                                 | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally invalidated)                                                                                                                                                                                                                                                                           | 8.6.1                            | Same as above                                                                                                          |
| 8.7 Split `parse_assign_after_target`           | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                                                                                                                                                                                                                                                                            | 8.6.x                            | This function only does dispatching                                                                                    |
| 8.8 Test completion                             | 8.8.1 Add associativity cases (**red then green**)                                                                                                                                                                                                                                                                                                | 8.2.x                            | C5                                                                                                                     |
|                                                 | 8.8.2 Cover four literal error paths                                                                                                                                                                                                                                                                                                              | —                                | Cover untested paths                                                                                                   |
|                                                 | 8.8.3 Add `pratt/tests/mod.rs` wiring self-check assertion                                                                                                                                                                                                                                                                                        | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6, changed to `parser/tests/`**)                  |
| 8.9 Parameter position form normalization (D53) | 8.9.1 Grammar production `Param ::= Identifier ':' TypeExpr \| TypeExpr`: bare identifier resolves in the type namespace, reports E if not resolvable; unnamed signatures require lambda headers to carry parameter names themselves (RFC-007:47 existing rule); RFC-010 form table and interface example (`(Surface)`) reconciled with the batch | 8.3.2 + 6.8.1                    | Corpus survey zero migration (std/corpus bare identifiers are all real types); new E code for unknown type identifiers |

**Risk**: 8.4 dual parser differential is a **non-skippable equivalence checkpoint**. If it produces
non-equivalence, it must be resolved before 8.5 stream switch, not papered over after the switch
with C5′. If the ~89 BP reference migrations in 8.6 are missed, **associativity changes silently**
(no error, only parsing result changes).

> **8.3–8.6 depends on D0/D2 in `08`**: if the grammar file is placed outside `parser/`, or the
> action code reverse-references `sema`, it will be blocked at `check-boundary.py`.

### P9 Anti-regression Gates (Unified Script List — supplemented by this document for original gap G3)

Source: [01](01-routing.md) §Anti-regression Mechanism + [08](08-maintenance-mechanism.md)
§Machine-checkable Rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts are added to CI at
the "introduction phase"; the report-only ones are uniformly switched to hard at P9 (changed from
"report" to "fail")—stock violations (parallel representations, disambiguation aliases, etc.) must
wait for P6/P8 to clear to zero; switching to hard prematurely will leave CI red for a long time and
the gate will be manually bypassed; when P9 switches to hard, the gate should be exactly all green.
P2's 2.5, P4's 4.3.2, and P7's 7c.1 all reference this table and do not create separate lists.

- [ ] **9.1–9.10 Ten scripts added to CI by introduction phase, P9 switches all to hard** (details
      in the table below)

| #    | Script                    | What it checks                                                                                                                  | Introduction Phase                                    | Acceptance                                                 |
| ---- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` bidirectional assertion                                                                      | P4                                                    | Deliberately omitting a stage must turn red                |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third file reads it                                                          | P4 (**sole attribution, no longer duplicated in P9**) | Deliberately leaving an unconsumed field must turn red     |
| 9.3  | `check-boundary.py`       | Forbid `include!`; `pub(crate)` cross-layer leaks may only decrease; forbid L2→L3 (**sole attribution**, defined by P0 of `08`) | **P0**                                                | Deliberately adding `include!` must turn red               |
| 9.4  | `check-test-wiring.py`    | Has `tests/` directory but parent module lacks `mod tests;` → fail                                                              | P1                                                    | Deliberately creating an orphan directory must turn red    |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message lacks `snapshot-update` marker                                                             | P2                                                    | Same as above                                              |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                            | P2                                                    | Same as above                                              |
| 9.7  | `check-corpus-parity.py`  | Corpus differential non-empty (diagnostic code+span compared item-by-item, **no grouping relaxation whatsoever**)               | P2                                                    | Same as above                                              |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction appears outside `synth.rs`                                                                             | P7                                                    | Same as above                                              |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representations, disambiguation aliases, synonym tables)                                        | **P0** (report-only, P9 switches to hard)             | Must report 3 sets of operator enums on unchanged code     |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                             | **P0**                                                | Deliberately adding a 6th entry-style wiring must turn red |

> **No line-count ratchet script** (`check-file-size.py` + `baseline.toml` was canceled on
> 2026-10-03). The scale problem is resolved by `08` Prohibition 2 Criterion A (responsibility
> separation, human judgment).

### P10 Other Cleanup and Status Corrections

Source: [06](06-cleanup-inventory.md) §Implementation Points

- [ ] **S2 Delete pure placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty-headed design documents** (1 level-3 task)
- [ ] **S4 RFC status corrections** (1 level-3 task)
- [ ] **S5 Reduce visibility of `pub` items** (1 level-3 task)
- [ ] **S6 opcode dead path cleanup** (1 level-3 task)

| Level 2                              | Level 3                                                                                                                                                                                                                                                                                                                                                          | Acceptance                                                                                                                                                |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S2 Delete pure placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladder / old syntax probe / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: `lib.rs:568` / `:582` two `_ => todo!()` change to meaningful degraded output**                                                                                                                          | `cargo test` passes; `clippy -D warnings` no new warnings                                                                                                 |
| S3 Empty-headed design documents     | After C1+C2 deletion, rewrite to point to `src/frontend/module/` (~2,670 lines); C3 rewrite boundary table; **synchronously clean up `config.js:259-268`**                                                                                                                                                                                                       | No 404 on the doc site; `check-docs-truth.py` passes                                                                                                      |
| S4 RFC status corrections            | C4 RFC-018 move back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 `TRACKING.md` add "Implementation Status" column (D36)                                                                                                                                                                                                                   | `check_tracking.py` exit code 0 (**do not manually edit TRACKING.md**)                                                                                    |
| S5 Reduce visibility of `pub` items  | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be preserved**)                                                                                                                                                                                                                                                                               | The compiler can point out the real dead code                                                                                                             |
| S6 opcode dead path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete, opcode value reclaim already landed in 7e.1 with version bump); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes `op` field); **F7+F8 opcode generation-time gate** (`tools/code-tables` extends opcode extractor ~30 lines + 5 fact expression single-source-ification + `size()` full opcode comparison test) | opcode fact 5-location expression converges to 1 authoritative source; `size()` comment inconsistent with actual encoding (`bytecode.rs:2181-2182`) fixed |

## Implementation Points: Dependencies and Parallelism

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 stop-bleeding channel (2026-10-05 ruling)**: P3's prerequisites narrow to **2.3.2 + 2.4.1 +
> 2.4.3** (multi-file corpus layer + two red criteria). The rest of P2 (2.1 IR verifier, 2.2
> snapshots, 2.3.1/2.3.3 differential and performance baseline) can proceed in parallel with P3.
> Stop-bleeding for correctness vulnerabilities is not blocked by criterion infrastructure
> construction; but **P4 must still wait for all of P2**—the unified Driver requires all three
> layers of criteria to be in place.

| Constraint         | Reason                                                                                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1            | The three prohibitions constrain every action in P1 (1.2.x is exactly adding `mod` declarations to `mod.rs`)                                             |
| P1 → P2            | Resurrecting tests will change test counts and corpus baseline                                                                                           |
| P2 (narrowed) → P3 | Vulnerability criteria must **first be written red** (2.4.1/2.4.3) + multi-file corpus layer (2.3.2) must exist, otherwise red tests have nowhere to run |
| P3 → P4            | Fix bugs before refactoring; reverse order would let bugs solidify into "established behavior" via the stage table                                       |
| P4 → P5            | `include!` refactoring and checker split touch the same files                                                                                            |
| P5 → P6            | Consecutive changes to the same file must be separated, otherwise regressions can't be bisected                                                          |
| P6 → P7 / P8       | If type representation isn't converged first, the new IR will grow into a third representation                                                           |
| P7/P8 → P9         | The snapshot baseline of 9.5 and the `pub(crate)` leak count of 9.3 only take their final form after P7/P8                                               |

### Parallel Groupings

| Group                               | Members                                                                            | Basis                                                                                                                                                                                                                                  |
| ----------------------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A (sole authorization from RFC)** | P7 ∥ P8                                                                            | Disjoint file sets: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                          |
| B                                   | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                      | Each independently adds `mod`, reverts file by file                                                                                                                                                                                    |
| C                                   | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                    | Three layers of criteria independent of each other                                                                                                                                                                                     |
| D                                   | 4.4 (proof layer wrap-up) ∥ entire P6                                              | `02` explicitly says S5 "should proceed independently of S1–S4"                                                                                                                                                                        |
| E                                   | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                              | All mutually independent after 4.1 completes                                                                                                                                                                                           |
| F                                   | 9.3 ∥ 9.9 ∥ 9.10 (the three scripts introduced in P0 can be developed in parallel) | The snapshot baseline of 9.5 must be taken last                                                                                                                                                                                        |
| G                                   | S3 ∥ S4 ∥ S5                                                                       | Disjoint file sets (verified 2026-10-05: S3 touches `docs/src/dev/design/check/` + `config.js`; S4 touches `docs/src/rfc/` + `check_tracking.py` generator; S5 touches two `pub` in `src/frontend/core/typecheck/`; pairwise disjoint) |

**Not parallelizable (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5 same
batch of files); 4.3.2 ∥ 9.2 (already unified to P4).

**Internal new constraints in the document**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial;
7a→7b→7c→7d strictly serial, and the `Phi` arm of 7b must precede 7d.

## Key Decisions and Rationale

- **P0 as a separate phase rather than merged into P9**—gates are "post-hoc checks", rules are
  "pre-judgment". P9's scripts cannot prevent "patch instead of refactor"; only D0–D4's review
  checklist can.
- **P9 as the sole authoritative CI script list**—the three parallel lists from the original gap G3
  prevented closure. After merging, the 10 scripts each belong to one phase, with the report-only
  and hard-switch timing explicitly queryable.
- **P5 steps supplemented by this document rather than merged into P4**—P4 is already the largest
  single-point risk; layering on the checker split would exceed the rollback granularity.
- **5.3.2 takes "zero caller-side changes" as hard acceptance**—the split must be independently
  revertable.

## Known Limitations and Risks

- **1.3.x (fix exposed defects) has no regression criterion**—only says "the fixed defects should
  not be rolled back", lacking a recording requirement for "what was fixed". **Execution
  convention**: every 1.3.x fix commit message must reference the fixed test name and defect
  description, as a post-hoc bisection reference.
- ~~**7b.1 granularity too coarse**~~ **Fixed (2026-10-05)**: split into 7b.1a / 7b.1b as two
  exclusive commits per DoD 6.
- **8.2.1 granularity too coarse**—~89 rename references; missing changes **silently change
  associativity**.
- ~~**D17 / D20 have no corresponding level-3 tasks**~~ **Fixed (2026-10-05)**: D17 lands as 7e.1,
  D20 lands as 4.5.1/4.5.2; D32/D34 opcode reclaim merged into 7e.1, deletion action deferred to P10
  S6.
- ~~**`2315` / `2341` data loss defects originally planned as "independent issue"**~~ **Fixed
  (2026-10-05)**: absorbed as 7e.2 / 7e.3. Exception tables (throw/try) are language core semantics,
  must not be left as implementation backlog.
- ~~**E2 (`lib.rs:568/582` `todo!()`) has no phase attribution**~~ **Fixed (2026-10-05)**: merged
  into P10 S2.
- ~~**F7 (opcode gate extractor) attribution ambiguous**~~ **Fixed (2026-10-05)**: merged into P10
  S6's generation-time gate task.

## Conflict Registration: All Resolved

> **The original "Conflict Registration" C1–C8 and open issues from various documents have all
> converged into [RFC-039 Resolution Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50).** This document retains no pending items. Item-by-item correspondence:

| Original # | Topic                                                        | Resolution                                                                                                                                                   |
| ---------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1         | `precedence_inline.rs` resurrected in P1, deleted in P8      | **Resurrected** (P1's premise is to resurrect tests with real assertions), P8's 8.6 deletes the file together with Pratt                                     |
| C2         | "Grammar-driven" commitment vs table lookup                  | **Fully implemented**: adopt LALRPOP, see `05` "Syntax: Full Grammar-Driven"                                                                                 |
| C3         | C6 category definition conflicts with S1 classification      | **S1 grouped under C6** (resurrection needs only regression tests, not equivalence criteria); C6 definition expanded to "pure deletion or pure resurrection" |
| C4         | `test_release_plan_spans_consumed` whitelist                 | **No whitelist**, difference set must be empty; if not, the `ReleasePlan` contract has a defect → switch to `PlanId` (D20/D41)                               |
| C5         | `Aggregation` two function internal differences not verified | **P4's 4.2.7 has listed this as a must-verify item**, confirm line-by-line during implementation                                                             |
| C6         | wasm playground fix needs P4's `ProgramKind`                 | **wasm path vulnerability fix merged into 4.2.8**; P3 only handles three native entry points                                                                 |
| C7         | `method_def_ordinals` attribution                            | **Belongs to 04** (P7 batch c), see D21                                                                                                                      |
| C8         | `upvalue_count` fix and `.42` version number                 | **Fix + `VERSION` upgrade from 4 to 5** (format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`), see D17                         |

## See Also

- [RFC-039 body](../../rfc/accepted/039-compiler-architecture.md) — the sole authority for phase
  sequence, DoD, and global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 criterion category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0 detailed design and D0–D4 decision
  procedure
- [01-routing.md](01-routing.md) — target directory structure (post-task form)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — design basis for each phase
