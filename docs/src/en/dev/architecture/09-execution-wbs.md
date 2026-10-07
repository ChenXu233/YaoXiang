# Multi-Level Work Breakdown Structure (WBS)

> **Subsidiary design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). It
> expands the RFC's stage sequence into a **three-level task list** that can be committed and
> verified independently.
>
> Stage order and acceptance gates are governed by RFC-039 as the sole authority; this document only
> breaks tasks down to executable granularity and records unresolved conflicts.

## Positioning and Scope

| Level       | Count       | Meaning                                                                 |
| ----------- | ----------- | ----------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Stages, one-to-one with RFC-039's stage sequence                        |
| **Level 2** | 45          | Task groups, from the "Implementation Points" sections of each document |
| **Level 3** | 122         | Independently committable actions                                       |

**This document covers**: task decomposition, dependencies, parallel groups, binding of acceptance
criteria.

**This document does not cover**: design rationale for tasks (see `02`–`08`), justification of stage
order (see RFC-039).

## Current Status: Gaps Found During Decomposition

While consolidating the "Implementation Points" of 8 documents into this table, **5 hard gaps in the
documentation system itself** were discovered. All have been converged:

| #   | Gap                                                                                                                                                                                                                                                                                                                                            | Convergence Result                                                                                                                                          |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no implementation steps**. RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents describes how to do it; `01-routing.md`'s directory tree mentioned an `annotations.rs` line, the only reference in the batch                                                                                     | P5 steps are filled in by this document (see below)                                                                                                         |
| G2  | **`include!` refactoring attributed to three places, landed in zero**. RFC lists under P4; `02` references itself; `06` also lists it in S7. But `02`'s change list (6 additions + 20 modifications) **does not include `semantic_tokens.rs`**                                                                                                 | Implementation steps belong to §P5 5.1 in this document (after P4 completes); RFC and 06 references now point here                                          |
| G3  | **Three coexisting CI script lists**. `RFC` lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists 4 different checks (all differently named); `04` also wants `check-synth-boundary.py`. After deduplication, **at least 9 new scripts with no unified list**; `check-obligations.py` is doubly attributed to P4 and P9 | Unified list in §P9 of this document (10 scripts, each attributed to one stage), other documents reference this table                                       |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it in "1005 lines to revive", the same document's B6 says it must be deleted together with the `Precedence` enum, `05` advocates deletion                                                                                                                                      | **Revive in P1, delete in P8 along with Pratt** (decision recorded as C1) — first let CI regain coverage, deletion happens with the dead ladder in P8's 8.6 |
| G5  | **L2 goal statement conflict**. RFC and this directory's index say "grammar-driven syntax", but `05` self-assesses "**did not adopt a true grammar-driven approach (LALRPOP), only achieved declarative table lookup**"                                                                                                                        | Converged to **LALRPOP full grammar-driven** (05 §Syntax: Full Grammar-Driven), G7/G8 acceptance gates executed per that commitment                         |

## Goal Design: Stage Sequence

```
P0  Repository maintenance mechanism and code placement rules     08        ← new, set rules first
P1  Revive orphan tests and fix defects           06 §S1
P2  Establish equivalence criterion baseline             07 all
P3  Fix correctness vulnerabilities (minimal scheme)        02 §S3-vulnerability half
P4  Stage contract and unified Driver          02
P5  checker file-internal split             02 + this document
P6  Type representation unification                 03
P7  Intermediate representation SSA conversion                04      } parallelizable
P8  Frontend paradigm change                   05      }
P9  Anti-rebound gates (including unified script list)     01 §Anti-rebound + 08 §Machine-checkable rules
P10 Remaining cleanup and status correction             06 §S2/S3/S4/S5/S6
```

**Reason for the new P0**: The three prohibitions (no fabrication / no infinite padding / patch
instead of refactor) constrain every action in P1–P10. Rules must be set before work begins;
otherwise, when P1 revives tests, people will already be adding `mod` declarations to
`tests/mod.rs`—which is exactly the cause of the G4 conflict.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [x] **0.1 Codification of rules** (3 level-3 tasks; 0.1.2/0.1.3 landed early on 2026-10-05; 0.1.1
      passed review on 2026-10-06: content verified + CONTRIBUTING.md fully translated, English
      version moved to docs/gh/CONTRIBUTING.en.md)
- [x] **0.2 Gate implementation** (3 level-3 tasks, completed 2026-10-06: check-concepts /
      check-fanout / check-boundary all integrated into CI in report-only mode (concepts job),
      negative verification probes all pass (newly added entry wiring red / L2→L3 reverse use red /
      unchanged code reports 3 sets of operator enums + 2 sets of parallel type representations
      including `ir::Type` alias), `pub(crate)` baseline 137, P9 unifies conversion to hard)
- [ ] **0.3 Table gate rollout** (1 level-3 task)

| Level 2                   | Level 3                                                                                                                                                                                                                                                                                                                                              | Prerequisite | Acceptance                                                                                                                                                                          |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Codification of rules | 0.1.1 Write D0–D4 and three prohibitions into `CONTRIBUTING.md`; responsibility categories reference the directory responsibility table from `01-routing.md`                                                                                                                                                                                         | —            | Manual review passed                                                                                                                                                                |
|                           | 0.1.2 Implementer touchpoints landing (**completed 2026-10-05**): create `AGENTS.md` (root entry) + `compiler-architecture/HOWTO.md` (work self-check + D3 patch determination); add "Responsibility Attribution and Decision Procedure" required block to PR template (D44); add "Code Placement and Change Procedure" section to `CONTRIBUTING.md` | —            | Landed; review verifies four touchpoints' content                                                                                                                                   |
|                           | 0.1.3 Rule body extraction (**completed 2026-10-05**): create `docs/src/dev/coding-rules.md` (precise rule description, long-term validity, not archived with RFC-039); 08 adds migration header note converted to diagnostic record; all four touchpoints now point to coding-rules                                                                 | —            | Landed                                                                                                                                                                              |
| 0.2 Gate implementation   | 0.2.1 `check-concepts.py` (prohibition 1 A/B/C/D)                                                                                                                                                                                                                                                                                                    | 0.1.1        | **Must report 3 sets of operator enums and 2 sets of parallel type representations (including `ir::Type` alias) on unchanged code**; report-only in this stage, P9 converts to hard |
|                           | 0.2.2 `check-fanout.py` (prohibition 3 A/B/C)                                                                                                                                                                                                                                                                                                        | 0.1.1        | **Deliberately adding a 6th entry-style wiring must be red**; report-only in this stage, P9 converts to hard                                                                        |
|                           | 0.2.3 `check-boundary.py` (prohibition 2 criterion C)                                                                                                                                                                                                                                                                                                | 0.1.1        | **Deliberately adding an L2→L3 reverse `use` must be red**; record initial value of `pub(crate)` leakage count                                                                      |
| 0.3 Table gate rollout    | 0.3.1 Extend `tools/code-tables` to cover opcode / type tables                                                                                                                                                                                                                                                                                       | 0.1.1        | Depends on P6's T1; separate closure                                                                                                                                                |

> **No size baseline task**. On 2026-10-03, all line count / volume gates were cancelled; size
> issues are solved by responsibility separation (see `08` prohibition 2 criterion A, manual
> judgment). **0.2.1 is the core acceptance for this stage**: if the gate cannot report the known
> parallel representations, the criterion design is wrong—it is a fake gate.

### P1 Revive Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Points S1 + [05](05-frontend-paradigm.md)
§Test Reconstruction

- [x] **1.1 Revive `lexer/tests/`** (4 level-3 tasks, completed 2026-10-06: 55 lexical tests online;
      delete 7 empty shells; `#[path]` bypass merged into main wiring to avoid duplicate runs)
- [x] **1.2 Revive 4 places in live directories** (completed 2026-10-06: precedence_inline 6 +
      overload_inline 7 + json 3 + template 7; additionally delete typecheck/tests/semantic_db.rs
      remnant 5 lines)
- [x] **1.3 Fix exposed defects** (completed 2026-10-06, **measured differently from prediction**:
      overflow paths and `\x`/`\u` escapes are currently safe, 20 probe tests added never-before
      coverage; the actually exposed defect is "illegal alphanumeric adjacency after base literals
      is silently split"—`0b102` compiles and runs with wrong value 2, `0o128` gives 10,
      `0x1FG`/`123abc` reports misleading E1001. Fix: add shared helper `reject_trailing_alnum` in
      `literals.rs` (prohibition 3 compliant: one implementation, five call sites), 5 red-then-green
      tests all green)

| Level 2                                 | Level 3                                                                                                                                                 | File:Line                  | Prerequisite | Acceptance                                                                   |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | ---------------------------------------------------------------------------- |
| 1.1 Revive `lexer/tests/`               | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                     | After `lexer/mod.rs:106`   | —            | Test count increases                                                         |
|                                         | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                             | `lexer/tests/mod.rs:15-25` | 1.1.1        | **Without this, 159 lines still don't run**                                  |
|                                         | 1.1.3 Delete 7 empty shell files + their `mod` declarations                                                                                             | `lexer/tests/` 7 files     | 1.1.1        | C6 has no criterion                                                          |
|                                         | 1.1.4 Delete `pub use ...::*;` re-export block                                                                                                          | `tests/mod.rs:28-38`       | 1.1.3        | That block is precisely what masks the empty shells                          |
| 1.2 Revive 4 places in live directories | 1.2.1–1.2.4 respectively wire `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` | Each parent module         | —            | Test count increases; **1.2.1 conflicts with 8.0.1, see conflict record C1** |
| 1.3 Fix exposed defects                 | 1.3.1 Four base scanners overflow paths                                                                                                                 | `literals.rs`              | 1.1.x        | `cargo test` green                                                           |
|                                         | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                    | `literals.rs`              | 1.1.x        | Same as above                                                                |

**Acceptance**: Test count increases; **real defects are expected to be exposed**, time must be
reserved for fixes. Revert per file, but **fixed defects should not be rolled back**.

### P2 Establish Equivalence Criterion Baseline

Source: [07](07-equivalence-oracle.md) all (that document has no "Implementation Points" section,
reverse-engineered from structure)

- [x] **2.1 IR static verifier** (3 level-3 tasks, all completed 2026-10-06: verify.rs dual-mode + 7
      invariants + corpus runs green, actually fixed three ir_gen defects)
- [x] **2.2 Normalized snapshots** (2 level-3 tasks, completed 2026-10-07: normalize.rs + 204
      snapshots in repository + gate 9.5 in CI)
- [x] **2.3 Corpus diff** (3 level-3 tasks, all completed 2026-10-07)
- [x] **2.4 Vulnerability-specific criteria** (3 level-3 tasks, all completed 2026-10-06:
      2.4.1/2.4.3 turn green with P3, 2.4.4 red state ready for P4/D20)
- [ ] **2.5 Regression gate** (1 level-3 task)

| Level 2                             | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Prerequisite | Acceptance                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------- |
| 2.1 IR static verifier              | 2.1.1 `verify.rs` dual-mode `verify_loose` / `verify_ssa` (**completed 2026-10-06**: `src/middle/core/verify.rs`, 07 §102 contract form `verify(ir, mode) -> Result<(), VerifyError>`, violations collected in full not stop at first error; def/use extraction exhaustive match with no wildcard arm, Phi variant introduction causes compile failure forcing additional check)                                                                                                                                                                                                                                                | —            | C4                                                                                      |
|                                     | 2.1.2 **`verify_loose` runs green on existing non-SSA IR** (**completed 2026-10-06**: `test_verify_loose_corpus_green` all 204 files/536 functions/11990 instructions green, non-empty assertion surface built-in; actually fixed three ir_gen defects per D38, corpus diff baseline self-comparison zero-diff proves behavioral equivalence)                                                                                                                                                                                                                                                                                   | 2.1.1        | **Hard threshold, admission condition for P7 batch a**—**unlocked**                     |
|                                     | 2.1.3 Implement 7 invariants (dominance / unique definition / Phi consistency / jump target / global out-of-bounds / type consistency / inner isolation) (**completed 2026-10-06**: judgment surface narrowed per actual measurement registration—dominance = named slot must-defined (temp slot Void pre-initialization is frame semantics, ir_gen intentionally depends on), type consistency = parameter slot signature + constant trusted source (ir_gen doesn't write slot type table, complete surface goes to P7 value table 04 §262), Phi waits for batch b variant introduction; see verify.rs file header and 06 §G6) | 2.1.1        | C4                                                                                      |
| 2.2 Normalized snapshots            | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / sort predecessors) (**completed 2026-10-07**: `src/middle/core/normalize.rs`, 07's five rules implemented item by item; actually fixed two issues—DefId relativized by first occurrence order (intern order is unstable across processes, 141/143 actually measured), Arg independent arg% prefix (sharing pool with Local loses variant distinction); instruction printing exhaustive match)                                                                                                                                                    | —            | C1/C3/C5                                                                                |
|                                     | 2.2.2 Snapshot ingestion + manual review process (`src/middle/core/tests/snapshots/`) (**completed 2026-10-07**: 204 snapshots ingested by corpus relative path mirroring; update process `UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored` + git diff review + same PR ingestion, see tests/snapshot.rs file header; tampering tested precisely red)                                                                                                                                                                                                                                                                 | 2.2.1        | C1/C3/C5                                                                                |
| 2.3 Corpus diff                     | 2.3.1 Diff framework + 293 corpus baseline (**completed 2026-10-06**: probe `examples/corpus_probe.rs` + baseline `tests/baselines/corpus-parity.jsonl` (322 entries) + gate `check-corpus-parity.py` in CI (hard gate); normalization includes pointer form scrub, two full runs gate self-comparison zero diff)                                                                                                                                                                                                                                                                                                               | —            | Diagnostics/exit code/stdout item by item                                               |
|                                     | 2.3.2 **Multi-file corpus layer** (create `tests/yaoxiang-multifile/`, project fixture with `yaoxiang.toml`—decision D48) (**completed 2026-10-06**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | —            | **Unconditional must-do (D40), the only executable behavioral criterion source for P4** |
|                                     | 2.3.3 C4 behavior diff required coverage list (10 semantic categories) + performance baseline (criterion smoke benchmark) (**completed 2026-10-07**: 10 semantic categories verified item by item with full coverage, 26 representative files 26/26 within corpus-parity.jsonl diff baseline; smoke benchmark `benches/pipeline.rs` first measurement—full corpus compile 17.6s / CLI cold start 169.7ms, interpreter throughput already covered by benches/lib.rs hotpath group)                                                                                                                                               | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                                  |
| 2.4 Vulnerability-specific criteria | 2.4.1 `test_multifile_proof_obligation_not_dropped` (**completed 2026-10-06: red state ready**, actually measured single file ["E4018"] vs compile_project [])                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 2.3.2        | **Must first be red**                                                                   |
|                                     | 2.4.3 `test_no_silent_pass_on_unproven` (**completed 2026-10-06: red state ready**, trigger source actually measured silent under compile_project)                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | —            | Block `checker.rs:5179/5318/5448`                                                       |
|                                     | 2.4.4 `test_release_plan_spans_consumed` (**completed 2026-10-06: red state ready**, actually measured 168 files 373 keys unconsumed—D20 contract defect evidence, contract fix goes to P4/D20)                                                                                                                                                                                                                                                                                                                                                                                                                                 | —            | **No whitelist, difference set must be empty (D41)**                                    |
| 2.5 Regression gate                 | 2.5.1–2.5.4 see P9 unified list                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | 2.1–2.3      | See P9                                                                                  |

> **P2 landing (2026-10-06, 2.1)**: verify.rs dual-mode + 7 invariants + full corpus runs green.
> **D38 prediction fulfilled**—during green runs, actually fixed three real defects in ir_gen (see
> 06 §G6 for details): method locals table truncated to parameter count (local_count underestimate,
> runtime relies on Frame::set_slot fallback), un-annotated parameters dropped by filter_map (three
> places, params metadata and frame layout mismatch), match result slot depends on frame implicit
> Void initialization (now explicitly pre-initialized). The actual semantic of jump target is
> flattened instruction index (not 07's text label keying), CFG built at instruction level—recorded
> in verify.rs file header. **Commit correspondence**: ir_gen fix = `a9ea199b`; verify main =
> `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its message to be mis-labeled as
> "docs(rfc): RFC-027a…", content actually verify.rs + six test files, buried in history middle
> unamendable, hereby recorded).
>
> **P2 narrow part landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleed
> channel prerequisite unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses the same `TestFileSpec` as single-file
>   corpus (single point on library side), binary positioning/subprocess spawning extracted to
>   `tests/common/` and yx_runner migrated in sync (prohibition 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses `SumUpTo(3, r)` form
>   nailed down by rfc027 test (return position/call site/binding site)—02's `Sorted(3)` annotation
>   example in current code would be substituted for evaluation and directly Disproved, not
>   presenting Unproven silent; 07's pseudocode `Sorted(3)` is only for illustration. Red state
>   actual measurement data in test file header.
> - The two red criteria hang `#[ignore]` per the 2.4.5 red skeleton precedent: default suite all
>   green, `-- --ignored` reproduces red state; **when P3 fix lands, remove attribute to turn
>   green**.
> - The intentional deviation from 07's (code, file, line) table in comparison normalization (remove
>   file/line, only compare error code sets) is documented in 2.4.1 test comment.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **removed from P2**: the `Program` / `Obligations` types they reference don't exist until P4, P2
> cannot compile. The former is P4's 4.1.4 (no double registration), the latter's `#[ignore]` red
> skeleton merged into 4.3.1.
>
> **2.3.3 landing (2026-10-07)**: C4 required coverage list verified item by item (07 §143), 10
> semantic categories fully covered at corpus layer and all within corpus-parity.jsonl diff baseline
> (26 representative files 26/26 hit, grep verified)—ref/borrow/move 05-ownership full directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure captures
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum type sum_type__+
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential type heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic diff
> surface), refinement constraints Drop sequence refined__ ×5 + 05-ownership driven ReleasePlan
> (span key contract has 2.4.4 red criterion standing by). Performance baseline
> `benches/pipeline.rs` ingested (07 §145's three smoke benchmarks of two):
> `corpus_compile_frontend` full corpus compile first measurement 17.6s, `cli_cold_start_hello`
> process cold start 169.7ms; interpreter throughput covered by benches/lib.rs hotpath group
> (interp_fib_recursive_27 / interp_loop_steady_10m), no duplicate construction. Regression gate
> check-perf-regression.sh goes to P9 unified list (2.5).

### P3 Fix Correctness Vulnerabilities (Minimal Scheme)

Source: [02](02-stage-contract.md) §Implementation Points S3's vulnerability fix half

- [x] **3.1 `proof_calls` consumer side** (2 level-3 tasks, completed 2026-10-06)
- [x] **3.2 Second silent drop point** (1 level-3 task, completed 2026-10-06)
- [x] **3.3 panic to diagnostic** (1 level-3 task, completed 2026-10-06)

| Level 2                         | Level 3                                               | File:Line                                | Prerequisite | Acceptance                          |
| ------------------------------- | ----------------------------------------------------- | ---------------------------------------- | ------------ | ----------------------------------- |
| 3.1 `proof_calls` consumer side | 3.1.1 Restrict `proof_calls` to private + only getter | `types.rs:29`                            | 2.4.1        | Vulnerability criterion turns green |
|                                 | 3.1.2 Complete three consumer points                  | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1        | C2                                  |
| 3.2 Second silent drop point    | 3.2.1 `Unproven` empty match arm produces diagnostic  | `checker.rs:1303-1314`                   | 2.4.3        | `test_no_silent_pass_on_unproven`   |
| 3.3 panic to diagnostic         | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic | `predicate.rs:34-36`                     | —            | No panic on Z3 missing              |

**Acceptance**: vulnerability criteria turn green, and **deliberately undoing the fix must turn red
again**. **Risk**: Adding E4018 to multi-file path is a breaking change.

> **P3 landing (2026-10-06)**: 3.1.1 `proof_calls` restricted to `pub(crate)` + only getter (fully
> private requires modifying 11 constructor literals, not taken in stop-bleed phase; closure when P4
> Obligations migrates); execution mechanism extracted to `src/frontend/proof_execution.rs`
> (prohibition 3 single point—cannot be placed in typecheck/, that would create L3→L4 reverse
> dependency); 3.1.2 per 02 evidence chain step 5 completed **four entry points** (WBS line number
> missed compile_project `:99`, must be completed and registered per 3.1.1 acceptance "criterion
> turns green"); 3.2.1 empty arm changed to bookkeeping (throw on proof_calls) + diagnostic
> (`into_result` established conversion path); 3.3.1 SOLVER slot Option-ified, Z3 missing gracefully
> degrades to `SMTResult::Unknown` (E2031 family diagnostic replaces process crash). Two
> vulnerability criteria remove `#[ignore]` and turn normal green, supplement corpus fixture
> `proof-obligation-honored`; check-fanout inventory migrated accordingly (wiring point
> 19=inventory). Actual measurement: criteria 2.4.1/2.4.3 turn green, full suite lib 2494 /
> integration 272 / corpus dual layer all green.
>
> **Stop-bleed channel (2026-10-05 decision)**: P3 does not have to wait for P2 to fully complete.
> The only hard prerequisites for 3.1/3.2 are **2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two
> red criteria)**; 2.1/2.2/2.3.1/2.3.3 (IR verifier, snapshots, single-file diff, performance
> baseline) can proceed in parallel with P3. The stop-bleed of correctness vulnerabilities
> (`Sorted(3)` silent pass) should not be blocked by snapshot infrastructure construction. 3.3
> (panic to diagnostic) has no prerequisite, can land at any time.

### P4 Stage Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Points S1–S5. **The fix in this stage and P3 must
be split into two commits.**

- [ ] **4.1 Declarative stage table** (5 level-3 tasks)
- [ ] **4.2 Entry consolidation** (8 level-3 tasks)
- [ ] **4.3 Obligation ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm cleanup** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ization (D20)** (2 level-3 tasks)
- [ ] **4.6 LSP semantic data pipeline unification (D55)** (1 level-3 task)

| Level 2                                       | Level 3                                                                                                                                                         | Prerequisite | Acceptance                                                                                                                                                          |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative stage table                   | 4.1.1 `stage.rs` (`Stage` variants + `Stage::ALL` + `StageScope`)                                                                                               | P0           | `test_program_stage_coverage`                                                                                                                                       |
|                                               | 4.1.2 `program.rs` / `unit.rs`                                                                                                                                  | 4.1.1        | Same as above                                                                                                                                                       |
|                                               | 4.1.3 `Pipeline::run` switch to Driver (`pipeline.rs:141-227`)                                                                                                  | 4.1.2        | **Single-file diagnostic set and exit code byte-identical**                                                                                                         |
|                                               | 4.1.4 Stage coverage assertion                                                                                                                                  | 4.1.3        | C2                                                                                                                                                                  |
|                                               | 4.1.5 Full corpus zero-diff                                                                                                                                     | 4.1.4        | Diagnostic/exit code/stdout all zero-diff (293 corpus)                                                                                                              |
| 4.2 Entry consolidation                       | 4.2.1 `compile_project` slimmed down to Program constructor (`orchestrator.rs:99-237`)                                                                          | 4.1.5        | Four entries diagnostic set unified identical                                                                                                                       |
|                                               | 4.2.2 `check_project` (`:273-399`)                                                                                                                              | 4.2.1        | Same as above                                                                                                                                                       |
|                                               | 4.2.3 `check_source_in_project` (`:450`)                                                                                                                        | 4.2.2        | Same as above                                                                                                                                                       |
|                                               | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                                                       | 4.2.3        | Same as above                                                                                                                                                       |
|                                               | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                                                  | 4.2.1        | `check` consistent inside and outside project                                                                                                                       |
|                                               | 4.2.6 Delete LSP manual stage sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                                                  | 4.2.1        | LSP and CLI diagnostic set same                                                                                                                                     |
|                                               | 4.2.7 `Aggregation` parameter-driven binary choice (`orchestrator.rs:486-494`)                                                                                  | 4.2.1        | **Verify line by line the internal differences of the two functions during implementation** (conflict record C5 has been adjudicated as required verification item) |
|                                               | 4.2.8 wasm switch to `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                                                              | 4.2.1        | See C6                                                                                                                                                              |
| 4.3 Obligation ledger                         | 4.3.1 `obligations.rs` + `assert_drained()` (new; including `#[ignore]` red skeleton first)                                                                     | 4.1.3        | `test_obligations_drained` turns green                                                                                                                              |
|                                               | 4.3.2 Obligation diagnostic W→E upgrade                                                                                                                         | 4.3.1        | Manual review each new E                                                                                                                                            |
| 4.4 Proof layer and wasm cleanup              | 4.4.1 `layers/README.md` layer order changed to actual order                                                                                                    | —            | **Independent of 4.1–4.3**                                                                                                                                          |
|                                               | 4.4.2 `default_solver()` changed to `&'static` singleton (`proof/smt/backend.rs:67-72`)                                                                         | —            | Cache hit rate observable                                                                                                                                           |
|                                               | 4.4.3 Add warning to degradation path (`checker.rs:1283-1293`)                                                                                                  | —            | —                                                                                                                                                                   |
| 4.5 Cross-layer contract PlanId-ization (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (`layers/ownership.rs:31` producer-side allocation + `ir_gen.rs:1942` consumer-side match)                            | 4.3.1        | `test_release_plan_spans_consumed` difference set empty (D41)                                                                                                       |
|                                               | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                                              | 4.5.1        | C2; span mismatch type silent failure to zero                                                                                                                       |
| 4.6 LSP semantic data pipeline unification    | 4.6.1 Project-internal file semantic data (SemanticDB) uses orchestrator's same pipeline as diagnostics, same source; cross-file references carry `resolves_to` | 4.2.1        | Project-internal cross-file jump hits definition; semantic data and diagnostics from same compilation                                                               |

### P5 checker File-Internal Split — Filled in by this document (Original Gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (`collect_used_in_type`'s
`pub(crate)` path must not change)

- [ ] **5.1 `include!` → real `mod`** (2 level-3 tasks)
- [ ] **5.2 Extract `refinement`** (2 level-3 tasks)
- [ ] **5.3 Extract `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                                                         | Prerequisite | Acceptance                                                     |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------- |
| 5.1 `include!` → real `mod` | 5.1.1 `checker.rs:5618`'s `include!` changed to real `mod` declaration                                                                                                                                                                                                          | P4 complete  | C1 zero-diff; `semantic_tokens.rs` gains module identity       |
|                             | 5.1.2 Add `use super::*` or per-item import (`include!` era implicit scope inheritance)                                                                                                                                                                                         | 5.1.1        | `cargo build` green                                            |
| 5.2 Extract `refinement`    | 5.2.1 Move out refinement block (currently `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                                                 | 5.1.1        | C1 zero-diff                                                   |
|                             | 5.2.2 `collect_refined_binding_checks` signature change: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                                            | 5.2.1        | C1; call point only `1235`                                     |
| 5.3 Extract `annotations`   | 5.3.1 Move out annotation checks (currently `3441-3661`, including `is_predicate_head`)                                                                                                                                                                                         | 5.2.2        | C1 zero-diff                                                   |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) remains `pub(crate)` and stays under `checker` module path** → add `pub(crate) use annotations::*;` at the top of `checker.rs`, so that the import in `inference/statements.rs:16` **does not change a single character** | 5.3.1        | **Zero call site changes** is the hard acceptance of this step |

**Hard constraint**: `collect_used_in_type` is used cross-module by `inference/statements.rs:16`.
The approach is `pub(crate) use` re-export, making the split a **pure relocation**.

**Only split confirmed large blocks.** Don't split modules of only 200–400 lines—that increases
navigation cost, reduces cohesion, yields zero decoupling benefit. Other submodules (signatures /
type_defs / imports) wait until the file grows back to 1,500 lines to split.

### P6 Type Representation Unification

Source: [03](03-type-unification.md) §Implementation Points stage 0–6, criterion C3. **Stage 0→6
strictly serial.**

- [ ] **6.0 Gate first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead variant deletion** (2 level-3 tasks)
- [ ] **6.4 parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variants** (2 level-3 tasks)
- [ ] **6.6 Gate to hard** (1 level-3 task)
- [ ] **6.7 Directory rename (D1, pure relocation batch)** (1 level-3 task)
- [ ] **6.8 Annotation check mode and Fn parameter representation (D54, added 2026-10-05)** (2
      level-3 tasks)

| Level 2                                                         | Level 3                                                                                                                                                                                                                                                                                                                                                                                           | File:Line                                                                     | Acceptance                                                                                                                                                                                                       |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.0 Gate first                                                  | 6.0.1 Create `tools/type-tables`, implement T1/T2/T3, **T1 first only report not fail**                                                                                                                                                                                                                                                                                                           | New crate                                                                     | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge reconstruction points listed separately**                                                                                           |
|                                                                 | 6.0.2 Take full baseline on unchanged code                                                                                                                                                                                                                                                                                                                                                        | —                                                                             | No code changes                                                                                                                                                                                                  |
| 6.1 Name normalization                                          | 6.1.1 Count non-canonical type name occurrences in corpus                                                                                                                                                                                                                                                                                                                                         | `tests/yaoxiang/`                                                             | Decide whether to directly delete synonyms                                                                                                                                                                       |
|                                                                 | 6.1.2 `from_builtin_name` cut synonyms → lexer alias table                                                                                                                                                                                                                                                                                                                                        | `mono.rs:618-643`                                                             | C3                                                                                                                                                                                                               |
|                                                                 | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                                                                                                                                                                                                              | `types.rs:162-165`                                                            | **Must verify line by line, cannot assume equivalence**                                                                                                                                                          |
| 6.2 Bytecode type unification                                   | 6.2.1 Three places change `MonoType`                                                                                                                                                                                                                                                                                                                                                              | `bytecode.rs:812/814/854`, `image.rs:44`                                      | C3 + `dump_bytecode` targeted comparison                                                                                                                                                                         |
|                                                                 | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                                                                                                                                                                                                          | `bytecode.rs:2352-2390`                                                       | Same as above                                                                                                                                                                                                    |
|                                                                 | 6.2.3 `type_table` element type change `MonoType`                                                                                                                                                                                                                                                                                                                                                 | `image.rs:44`                                                                 | Same as above                                                                                                                                                                                                    |
| 6.3 Dead variant deletion                                       | 6.3.1 Delete 11 variants + clear match arms + fallback                                                                                                                                                                                                                                                                                                                                            | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                                   | C3 + `cargo build` green                                                                                                                                                                                         |
|                                                                 | 6.3.2 **Must be in same batch as 6.3.1** rewrite reverse bridge                                                                                                                                                                                                                                                                                                                                   | `passes/mono/function.rs:507-529` (`mono_to_ast_type` reconstructs 7 of them) | Generic substitution targeted comparison, otherwise **silently change behavior**                                                                                                                                 |
| 6.4 parser data flow                                            | 6.4.1 Add `probe.rs`                                                                                                                                                                                                                                                                                                                                                                              | New                                                                           | C3                                                                                                                                                                                                               |
|                                                                 | 6.4.2 Change `parser_state.rs:46-58`                                                                                                                                                                                                                                                                                                                                                              | —                                                                             | Same as above                                                                                                                                                                                                    |
|                                                                 | 6.4.3 Delete `operator_interfaces::spec()` calls (**2 places**: `declarations.rs:509`, `ast.rs:930`)                                                                                                                                                                                                                                                                                              | —                                                                             | Same as above                                                                                                                                                                                                    |
|                                                                 | 6.4.4 Merge two `name_used_as_type*`                                                                                                                                                                                                                                                                                                                                                              | `ast.rs:847` + `declarations.rs:42`                                           | Same as above                                                                                                                                                                                                    |
|                                                                 | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) wired through `NameKind::Builtin`                                                                                                                                                                                                                                                                                                                        | —                                                                             | Same as above                                                                                                                                                                                                    |
| 6.5 AST dead variants                                           | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                                                                                                                                                                                                                        | `ast.rs:37-43` + 4 consumer places + 2 exhaustive arms                        | `tests/integration/` 18 modules green (**split 2 commits**)                                                                                                                                                      |
|                                                                 | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                                                                                                                                                                                                                   | `ast.rs:241-249`                                                              | Note **semantic risk**                                                                                                                                                                                           |
| 6.6 Gate to hard                                                | 6.6.1 T1 from warning to `panic!`                                                                                                                                                                                                                                                                                                                                                                 | `tools/type-tables`                                                           | **Deliberately introducing zero-construction variant must be red**                                                                                                                                               |
| 6.7 Directory rename (D1, pure relocation batch)                | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; `parser/ast.rs` moved to top-level `ast/`; bytecode domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` moved to `middle/bytecode/`                                                                                                                                                                                             | Whole repo                                                                    | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline no regression                                                                                                                          |
| 6.8 Annotation check mode and Fn parameter representation (D54) | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>` (name and type co-located), closed together with 6.5.2 deletion of `Assign.signature_params`—6.5.2 only deletes old representation, this item provides replacement representation                                                                                                                                                               | `ast.rs:451-454`, consumer side ~20 files (mechanical change)                 | C3 (pure representation reshaping batch, compared against unused); split into independent commit                                                                                                                 |
|                                                                 | 6.8.2 Declaration-driven check mode: parameter types drive lambda header by position (independent of name), exit (tail expression and return) uniformly compared against annotation return type, non-lambda value infer+unify (isomorphic with `m: Int = "hello"`); red criterion first—u1–u4/v1–v5 probes into corpus (red first then green); delete `declarations.rs:337-376` parser name merge | `statements.rs:1170-1914`                                                     | Comparison-enabled batch is **behavior fix**: new diagnostic codes, baseline update, C3 not applicable; v1–v5 five syntactic forms same judgment (eliminate "same semantics different syntax one red one green") |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA Conversion

Source: [04](04-ssa.md) §Implementation Points batch a–d, criterion C4, **strictly serial within
batch**.

- [ ] **7a Cut off multiple definitions** (3 level-3 tasks)
- [ ] **7b SSA form switch** (6 level-3 tasks; 7b.1 already split into 1a/1b per DoD 6)
- [ ] **7c Implicit contract explicitation** (3 level-3 tasks)
- [ ] **7d Last to do** (2 level-3 tasks)
- [ ] **7e `.42` data loss fix** (3 level-3 tasks; 2315/2341 absorbed from "independent issue",
      2026-10-05)

| Level 2                            | Level 3                                                                                                                                          | Prerequisite                        | Acceptance                                   |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------- | -------------------------------------------- |
| 7a Cut off multiple definitions    | 7a.1 Delete 3 statement-level reclaimers                                                                                                         | **2.1.2 `verify_loose` runs green** | C4                                           |
|                                    | 7a.2 6 save/restore replaced with RAII guard                                                                                                     | 7a.1                                | C1 snapshot zero-diff                        |
|                                    | 7a.3 Caliber unification (`1798` vs `2001`)                                                                                                      | 7a.2                                | C1                                           |
| 7b SSA form switch                 | 7b.1a `ir.rs` structure change #1-2 (`Operand` variant convergence + `Instruction::Phi` introduction)                                            | 7a.3 + **P6 complete**              | C4; exclusive commit                         |
|                                    | 7b.1b `ir.rs` structure change #3-5 (`FunctionIR` / `BasicBlock` / type annotation fields)                                                       | 7b.1a                               | C4; exclusive commit                         |
|                                    | 7b.2 `next_temp_reg` change to `Operand::Value`                                                                                                  | 7b.1                                | Same as above                                |
|                                    | 7b.3 `translator.rs` add `Phi` arm                                                                                                               | 7b.1                                | C4 + `.42` round-trip test                   |
|                                    | 7b.4 Linear scan allocator (within u8/255 slot model)                                                                                            | 7b.3                                | Same as above (+500~1000 lines)              |
| 7c Implicit contract explicitation | 7c.1 `synth.rs` boundary + boundary check script                                                                                                 | 7b.4                                | C4 + new script in CI                        |
|                                    | 7c.2 span consumption count                                                                                                                      | 7c.1                                | Consumption count **zero trigger**           |
|                                    | 7c.3 `method_def_ordinals` read-only                                                                                                             | 7c.1                                | Attribution questionable (C7)                |
| 7d Last to do                      | 7d.1 `generate_call_expr_ir` split into `CallArgs` + 6 `emit_*`                                                                                  | 7c.x                                | C1 snapshot zero-diff                        |
|                                    | 7d.2 "Pad 0" fallback changed to return diagnostic                                                                                               | 7d.1                                | C4                                           |
| 7e `.42` data loss fix             | 7e.1 `upvalue_count: 0` fix (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + already allocated unused opcode reclaim (D32/D34 with version bump)     | 7d.x                                | C4 + `.42` round-trip test; exclusive commit |
|                                    | 7e.2 `exception_handlers` persistence (`bytecode.rs:2315`—throw/try are language core semantics, **must not be left for future implementation**) | 7e.1                                | `.42` direct-run exception case diff         |
|                                    | 7e.3 `globals` persistence (`bytecode.rs:2341`)                                                                                                  | 7e.1                                | `.42` direct-run global variable case diff   |

**Expectation management**: **net increase of 900–1600 lines**. If the project is launched with
"shorter code" as the success criterion, this stage will be judged as failed.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Points stage 0–4, criterion C5.

- [ ] **8.0 Dead ladder cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexical convergence** (2 level-3 tasks)
- [ ] **8.3 Build LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual parser diff** (1 level-3 task)
- [ ] **8.5 Cutover** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 `parse_assign_after_target` split** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)
- [ ] **8.9 Parameter position form unification (D53, added 2026-10-05)** (1 level-3 task)

| Level 2                                       | Level 3                                                                                                                                                                                                                                                                                                              | Prerequisite                     | Acceptance                                                                                                        |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead ladder cleanup (C6)                  | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                                                                                                                                                                                                                                      | **Conflicts with 1.2.1, see C1** | C6 has no criterion                                                                                               |
|                                               | 8.0.2 Bare magic numbers named (`(6,7)` / `(11,1)` / `12`)                                                                                                                                                                                                                                                           | —                                | Same as above                                                                                                     |
| 8.2 Lexical convergence                       | 8.2.1 Four base scanners merge + three escape merges + multi-line string merge                                                                                                                                                                                                                                       | —                                | C5; `literals.rs` reduces ~500 lines                                                                              |
|                                               | 8.2.2 f-string nested compilation elimination                                                                                                                                                                                                                                                                        | 8.2.1                            | **span must change**, baseline must be built first                                                                |
| 8.3 Build LALRPOP grammar                     | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                                                                                                                                                                                                                                        | 8.2.x                            | `cargo build` passes; can produce AST for 293 corpus (**no comparison**)                                          |
|                                               | 8.3.2 Action code `grammar/actions.rs` (22 `Expr` variants each with one function)                                                                                                                                                                                                                                   | 8.3.1                            | Same as above                                                                                                     |
|                                               | 8.3.3 Error productions (preserve `Expr::Error` / `StmtKind::Error` placeholder semantics)                                                                                                                                                                                                                           | 8.3.2                            | Same as above                                                                                                     |
| 8.4 **Dual parser diff**                      | 8.4.1 Both parsers run 293 corpus + `src/std/tests`, AST normalized then **bit-by-bit compared**                                                                                                                                                                                                                     | 8.3.3                            | **All valid program ASTs fully equivalent—this is the equivalence proof for the entire grammar migration**        |
| 8.5 Cutover                                   | 8.5.1 `parse()` changed to call LALRPOP, Pratt kept as `parse_legacy()`                                                                                                                                                                                                                                              | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span identical per item (**no relaxation**, no C5′ exists)      |
| 8.6 Delete Pratt                              | 8.6.1 Delete `nud.rs`(1326) + `led.rs`(451) + two sets of BP ladders + ~89 references                                                                                                                                                                                                                                | 8.5.1                            | Corpus all green; `git grep BP_` zero hits                                                                        |
|                                               | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally obsolete)                                                                                                                                                                                                                                                 | 8.6.1                            | Same as above                                                                                                     |
| 8.7 `parse_assign_after_target` split         | 8.7.1 Split 8 responsibility segments + extract `skip_balanced_parens`                                                                                                                                                                                                                                               | 8.6.x                            | That function only does dispatch                                                                                  |
| 8.8 Test completion                           | 8.8.1 Add associativity test cases (**red first then green**)                                                                                                                                                                                                                                                        | 8.2.x                            | C5                                                                                                                |
|                                               | 8.8.2 Add four literal error paths                                                                                                                                                                                                                                                                                   | —                                | Cover untested paths                                                                                              |
|                                               | 8.8.3 Add `pratt/tests/mod.rs` wiring self-check assertion                                                                                                                                                                                                                                                           | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6, changed to `parser/tests/`**)             |
| 8.9 Parameter position form unification (D53) | 8.9.1 Grammar production `Param ::= Identifier ':' TypeExpr \| TypeExpr`: bare identifier resolved in type namespace, unresolved reports E; unnamed signatures require lambda header to carry parameter names (RFC-007:47 existing rule); RFC-010 form table and interface example (`(Surface)`) reconciled in batch | 8.3.2 + 6.8.1                    | Corpus survey zero migration (std/corpus bare identifiers are all real types); unknown type identifier new E code |

**Risk**: 8.4 dual parser diff is **a non-skippable equivalence gate**. If it produces
inequivalence, it must be resolved before 8.5 cutover, not covered up by C5′ after cutover. The ~89
BP reference migrations in 8.6 if missed will **silently change associativity** (no error, only
parse result changes).

> **8.3–8.6 depend on D0/D2 of `08`**: if the grammar file is placed outside `parser/`, or action
> code references `sema` in reverse, it will be blocked at `check-boundary.py`.

### P9 Anti-rebound Gates (Unified Script List — Filled in by this document Original Gap G3)

Source: [01](01-routing.md) §Anti-rebound mechanism + [08](08-maintenance-mechanism.md)
§Machine-checkable rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts are integrated into
CI in their "introduction stage"; those marked report-only are uniformly converted to hard in P9
(from "report" to "fail")—stock violations (parallel representations, disambiguation aliases, etc.)
need to wait for P6/P8 to zero out, converting to hard early will keep CI red long-term, gates
bypassed manually, when P9 converts to hard, gates should be exactly all green. P2's 2.5, P4's
4.3.2, P7's 7c.1 all reference this table, no separate lists.

- [ ] **9.1–9.10 Ten scripts integrated into CI per introduction stage, P9 uniformly converts to
      hard** (details in table below)

| #    | Script                    | What it checks                                                                                                                          | Introduction Stage                                  | Acceptance                                               |
| ---- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` bidirectional assertion                                                                              | P4                                                  | Deliberately missing one stage must be red               |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third file reads it                                                                  | P4 (**sole attribution, no longer repeated in P9**) | Deliberately leaving an unconsumed field must be red     |
| 9.3  | `check-boundary.py`       | Forbid `include!`; `pub(crate)` cross-layer leakage only allowed to decrease; forbid L2→L3 (**sole attribution**, `08`'s P0 defined it) | **P0**                                              | Deliberately adding `include!` must be red               |
| 9.4  | `check-test-wiring.py`    | Has `tests/` directory but parent module lacks `mod tests;` → fail                                                                      | P1                                                  | Deliberately creating orphan directory must be red       |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message lacks `snapshot-update` mark                                                                       | P2                                                  | Same as above                                            |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                                    | P2                                                  | Same as above                                            |
| 9.7  | `check-corpus-parity.py`  | Corpus diff non-empty (diagnostic code+span per-item comparison, **no grouping relaxation whatsoever**)                                 | P2                                                  | Same as above                                            |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction outside `synth.rs`                                                                                             | P7                                                  | Same as above                                            |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representation, disambiguation alias, synonym table)                                                    | **P0** (report-only, P9 converts to hard)           | Must report 3 sets of operator enums on unchanged code   |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                                     | **P0**                                              | Deliberately adding a 6th entry-style wiring must be red |

> **No line count ratchet script** (`check-file-size.py` + `baseline.toml` cancelled on 2026-10-03).
> Size issues are solved by `08` prohibition 2 criterion A (responsibility separation, manual
> judgment).

### P10 Remaining Cleanup and Status Correction

Source: [06](06-cleanup-inventory.md) §Implementation Points

- [ ] **S2 Delete pure placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty-headed design document** (1 level-3 task)
- [ ] **S4 RFC status correction** (1 level-3 task)
- [ ] **S5 `pub` items visibility reduction** (1 level-3 task)
- [ ] **S6 opcode dead path cleanup** (1 level-3 task)

| Level 2                              | Level 3                                                                                                                                                                                                                                                                                                                                                | Acceptance                                                                                                                                        |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| S2 Delete pure placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladder / old syntax probe / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: `lib.rs:568` / `:582` two `_ => todo!()` changed to meaningful degradation output**                                                                                                            | `cargo test` passes; `clippy -D warnings` no new warnings                                                                                         |
| S3 Empty-headed design document      | After C1+C2 deletion rewrite to point to `src/frontend/module/` (~2,670 lines); C3 rewrite boundary table; **synchronously clean up `config.js:259-268`**                                                                                                                                                                                              | Doc site no 404; `check-docs-truth.py` passes                                                                                                     |
| S4 RFC status correction             | C4 RFC-018 move back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 `TRACKING.md` add "Implementation Status" column (D36)                                                                                                                                                                                                         | `check_tracking.py` exit code 0 (**don't manually edit TRACKING.md**)                                                                             |
| S5 `pub` items visibility reduction  | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be preserved**)                                                                                                                                                                                                                                                                     | Compiler can point out real dead code                                                                                                             |
| S6 opcode dead path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete, opcode value reclaim already landed in 7e.1 with version bump); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes `op` field); **F7+F8 opcode generation-time gate** (`tools/code-tables` extends opcode extractor ~30 lines + 5 fact expression single-source + `size()` full opcode comparison test) | opcode fact 5 expressions converged to 1 authoritative source; `size()` comment inconsistent with actual encoding (`bytecode.rs:2181-2182`) fixed |

## Implementation Points: Dependencies and Parallelism

### Forced Serial Main Chain

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 stop-bleed channel (2026-10-05 decision)**: P3's prerequisites narrowed to **2.3.2 + 2.4.1 +
> 2.4.3** (multi-file corpus layer + two red criteria). The rest of P2 (2.1 IR verifier, 2.2
> snapshots, 2.3.1/2.3.3 diff and performance baseline) can proceed in parallel with P3. The
> stop-bleed of correctness vulnerabilities is not blocked by criterion infrastructure construction;
> but **P4 must still wait for P2 to fully complete**—unified Driver needs all three-layer criteria
> in place.

| Constraint         | Reason                                                                                                                                       |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1            | The three prohibitions constrain every action in P1 (1.2.x is exactly adding `mod` declarations to `mod.rs`)                                 |
| P1 → P2            | Reviving tests will change test count and corpus baseline                                                                                    |
| P2 (narrowed) → P3 | Vulnerability criteria must be **written red first** (2.4.1/2.4.3) + multi-file corpus layer (2.3.2) exists, red tests have somewhere to run |
| P3 → P4            | Fix bug then refactor; reverse order will let bugs be solidified as "established behavior" by the stage table                                |
| P4 → P5            | `include!` refactoring and checker split touch the same file                                                                                 |
| P5 → P6            | Consecutive changes to the same file must be separated, otherwise regression cannot be bisected                                              |
| P6 → P7 / P8       | If type representation is not converged first, the new IR will grow into a third set of representations                                      |
| P7/P8 → P9         | 9.5's snapshot baseline and 9.3's `pub(crate)` leakage count take final form only after P7/P8                                                |

### Parallelizable Groups

| Group                              | Members                                                                        | Basis                                                                                                                                                                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A (sole one authorized by RFC)** | P7 ∥ P8                                                                        | Disjoint file sets: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                       |
| B                                  | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                  | Each independent `mod` addition, per-file revert                                                                                                                                                                                    |
| C                                  | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                | Three-layer criteria independent of each other                                                                                                                                                                                      |
| D                                  | 4.4 (proof layer cleanup) ∥ entire P6 line                                     | `02` explicitly states S5 "should proceed independently of S1-S4"                                                                                                                                                                   |
| E                                  | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                          | All mutually independent after 4.1 complete                                                                                                                                                                                         |
| F                                  | 9.3 ∥ 9.9 ∥ 9.10 (three scripts introduced by P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                                                                                                                                                          |
| G                                  | S3 ∥ S4 ∥ S5                                                                   | Disjoint file sets (verified 2026-10-05: S3 touches `docs/src/dev/design/check/` + `config.js`; S4 touches `docs/src/rfc/` + `check_tracking.py` generator; S5 touches `src/frontend/core/typecheck/` two `pub`; pairwise disjoint) |

**Not parallelizable (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5 same
batch file); 4.3.2 ∥ 9.2 (unified to P4).

**Internal document added constraint**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial; 7a→7b→7c→7d
strictly serial, and 7b's `Phi` arm must precede 7d.

## Key Decisions and Rationale

- **P0 is an independent stage rather than merged into P9**—gates are "post-hoc checks", rules are
  "prior judgment". P9's scripts cannot prevent "patch instead of refactor", only D0–D4's review
  checklist can.
- **P9 as the sole authoritative list of CI scripts**—the three coexisting lists from original gap
  G3 made it impossible to close. After this table merges, 10 scripts each belong to one stage,
  report-only and conversion to hard timing explicitly checkable.
- **P5's steps are filled in by this document rather than merged into P4**—P4 is already the largest
  single-point risk, layering on checker split would exceed rollback granularity.
- **5.3.2 with "zero call site changes" as hard acceptance**—the split must be independently
  revertable.

## Known Limitations and Risks

- **1.3.x (fix exposed defects) has no regression criterion**—only says "fixed defects should not be
  rolled back", lacks recording requirements of "what was fixed". **Execution convention**: each
  1.3.x fix's commit message must reference the fixed test name and defect description, as post-hoc
  bisection basis.
- ~~**7b.1 granularity too coarse**~~ **Fixed (2026-10-05)**: split into 7b.1a / 7b.1b two exclusive
  commits per DoD 6.
- **8.2.1 granularity too coarse**—~89 rename references, missed changes **silently change
  associativity**.
- ~~**D17 / D20 have no corresponding level-3 tasks**~~ **Fixed (2026-10-05)**: D17 landed as 7e.1,
  D20 landed as 4.5.1/4.5.2; D32/D34's opcode reclaim merged into 7e.1, deletion action left for P10
  S6.
- ~~**`2315` / `2341` data loss defects originally "independent issue"**~~ **Fixed (2026-10-05)**:
  absorbed as 7e.2 / 7e.3. Exception table (throw/try) is language core semantics, must not be left
  as future implementation legacy.
- ~~**E2 (`lib.rs:568/582` `todo!()`) has no stage attribution**~~ **Fixed (2026-10-05)**: merged
  into P10 S2.
- ~~**F7 (opcode gate extractor) attribution ambiguous**~~ **Fixed (2026-10-05)**: merged into P10
  S6's generation-time gate task.

## Conflict Registry: All Adjudicated

> **Original "Conflict Registry" C1–C8 and open questions from each document have all converged to
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).** This
> document retains no pending items. Item-by-item correspondence:

| Original # | Issue                                                      | Adjudication                                                                                                                                          |
| ---------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1         | `precedence_inline.rs` revive in P1, delete in P8          | **Revive** (P1's premise is reviving tests with real assertions), P8's 8.6 deletes this file along with Pratt                                         |
| C2         | "Grammar-driven" promise vs table lookup                   | **Fully done**: adopt LALRPOP, see `05` "Syntax: Full Grammar-Driven"                                                                                 |
| C3         | C6 category definition conflicts with S1 classification    | **S1 assigned to C6** (revival does not need equivalence criterion, only regression tests); C6 definition expanded to "pure deletion or pure revival" |
| C4         | `test_release_plan_spans_consumed` whitelist               | **No whitelist**, difference set must be empty; cannot be empty means `ReleasePlan` contract has defect → change to `PlanId` (D20/D41)                |
| C5         | `Aggregation` two function internal differences unverified | **P4's 4.2.7 has listed it as required verification item**, verify line by line during implementation                                                 |
| C6         | wasm playground fix needs P4's `ProgramKind`               | **wasm path's vulnerability fix merged into 4.2.8**; P3 only handles three native entries                                                             |
| C7         | `method_def_ordinals` attribution                          | **Goes to 04** (P7 batch c), see D21                                                                                                                  |
| C8         | `upvalue_count` fix and `.42` version number               | **Fix + `VERSION` bump from 4 to 5** (format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`), see D17                     |

## See Also

- [RFC-039 Main Text](../../rfc/accepted/039-compiler-architecture.md) — sole authority for stage
  sequence, DoD, global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 criterion category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0 detailed design and D0–D4 decision
  procedure
- [01-routing.md](01-routing.md) — target directory structure (post-task-completion form)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — design basis for each stage
