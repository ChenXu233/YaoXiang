# Multi-level Work Breakdown Structure (WBS)

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). It
> expands the RFC's phase sequence into **three-level tasks that can be independently committed and
> independently verified**.
>
> Phase order and acceptance gates are governed by RFC-039 as the sole authority; this document only
> breaks the work down to an executable granularity and registers conflicts that have not yet been
> adjudicated.

## Position and Scope

| Level       | Count       | Meaning                                                                                         |
| ----------- | ----------- | ----------------------------------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Phases, one-to-one with the phase sequence in RFC-039                                           |
| **Level 2** | 46          | Task groups, from each document's "Implementation Notes" + 2026-10-07 audit addendum (P3's 3.4) |
| **Level 3** | 130         | Independently committable actions                                                               |

**This document covers**: task breakdown, dependencies, parallel grouping, and acceptance criteria
binding.

**This document does not cover**: task design rationale (see `02`–`08`), phase order justification
(see RFC-039).

## Current State: Gaps Discovered During Breakdown

When consolidating the "Implementation Notes" of the 8 documents into this table, **5 hard gaps were
found in the documentation system itself**. All have been converged:

| #   | Gap                                                                                                                                                                                                                                                                                                                  | Convergence Result                                                                                                                                                      |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no construction steps at all**. RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents describe how this split should be done; `01-routing.md`'s directory tree mentions `annotations.rs` once, the only reference in the entire batch                                   | P5 steps are filled in by this document (see below)                                                                                                                     |
| G2  | **`include!` refactoring has three claimed owners, zero landings**. RFC lists it in P4; `02` claims it; `06`'s S7 also lists it. But `02`'s change list (6 additions + 20 modifications) does **not include `semantic_tokens.rs`**                                                                                   | Construction steps belong to this document §P5 5.1 (after P4 prerequisite); RFC and 06's references now point here                                                      |
| G3  | **Three coexisting CI script lists**. `RFC` lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists 4 different checks; `04` also requires `check-synth-boundary.py`. After deduplication, **at least 9 new scripts with no unified list**; `check-obligations.py` is doubly owned in P4 and P9 | Unified list in this document §P9 (10 scripts, each owned by one phase); other documents reference this table                                                           |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it among "1005 lines to resurrect", but the same document's B6 also says it and the `Precedence` enum "must be deleted together"; `05` argues for deletion                                                                                           | **Resurrect in P1, delete with Pratt in P8** (decision registered as C1) — first let CI regain coverage, the deletion happens together with the dead steps in 8.6 of P8 |
| G5  | **L2 target description conflict**. RFC and this directory's index say "grammar-driven syntax", but `05`'s self-evaluation says "**did not adopt true grammar-driven (LALRPOP), only achieved declarative table lookup**"                                                                                            | Converged to **LALRPOP complete grammar-driven** (`05` §Syntax: Complete Grammar-Driven); G7/G8 acceptance gates execute per that commitment                            |

## Target Design: Phase Sequence

```
P0  Repository maintenance mechanism and code placement rules     08        ← New, rules first
P1  Resurrect orphan tests and fix defects         06 §S1
P2  Establish equivalence oracle baseline         07 entire
P3  Fix correctness vulnerabilities (minimal solution)        02 §S3-vulnerability half
P4  Phase contract and unified Driver          02
P5  checker file-internal split             02 + filled in here
P6  Type representation unification                 03
P7  Intermediate representation SSA-ification                04      } parallelizable
P8  Frontend paradigm change                   05      }
P9  Anti-rebound gates (with unified script list)     01 §Anti-rebound + 08 §machine-checkable rules
P10 Other cleanup and status corrections             06 §S2/S3/S4/S5/S6
```

**Reason for the new P0**: Three prohibitions (no fabrication / no endless padding / should refactor
but applied a patch instead) constrain every action in P1–P10. The rules must be established before
work begins, otherwise when P1 resurrects tests it will already be adding `mod` declarations to
`tests/mod.rs` — which is exactly the cause of conflict G4.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [x] **0.1 Rules written down** (3 level-3 tasks; 0.1.2/0.1.3 landed early on 2026-10-05; 0.1.1
      passed review on 2026-10-06: content verified + CONTRIBUTING.md fully localized to Chinese,
      English version moved to docs/gh/CONTRIBUTING.en.md)
- [x] **0.2 Gate implementation** (3 level-3 tasks, completed 2026-10-06: check-concepts /
      check-fanout / check-boundary all integrated into CI as report-only (concepts job), negative
      acceptance probes all pass (newly added entry wiring red / L2→L3 reverse use red / unchanged
      code reports 3 sets of operator enums + 2 sets of parallel type representations including
      `ir::Type` alias), pub(crate) baseline 137, unified to hard in P9)
- [ ] **0.3 Gate rollout** (1 level-3 task)

| Level 2                 | Level 3                                                                                                                                                                                                                                                                                                                                              | Prerequisite | Acceptance                                                                                                                                                                        |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Rules written down  | 0.1.1 Write D0–D4 and three prohibitions into `CONTRIBUTING.md`; responsibility categories reference `01-routing.md`'s directory responsibility table                                                                                                                                                                                                | —            | Manual review passes                                                                                                                                                              |
|                         | 0.1.2 Implementer touchpoints landed (**completed 2026-10-05**): created `AGENTS.md` (root entry) + `compiler-architecture/HOWTO.md` (pre-work self-check + D3 patch determination); PR template adds required block "responsibility attribution and decision procedure" (D44); `CONTRIBUTING.md` adds "Code Placement and Change Procedure" section | —            | Landed; review checks the four touchpoints                                                                                                                                        |
|                         | 0.1.3 Rule body extraction (**completed 2026-10-05**): created `docs/src/dev/coding-rules.md` (precise rule description, long-term valid, not archived with RFC-039); 08 adds migration header note converted to diagnostic record; all four touchpoints now point to coding-rules                                                                   | —            | Landed                                                                                                                                                                            |
| 0.2 Gate implementation | 0.2.1 `check-concepts.py` (prohibitions 1 A/B/C/D)                                                                                                                                                                                                                                                                                                   | 0.1.1        | **Must report 3 sets of operator enums and 2 sets of parallel type representations (including `ir::Type` alias) on unchanged code**; report-only in this phase, hard-closed in P9 |
|                         | 0.2.2 `check-fanout.py` (prohibitions 3 A/B/C)                                                                                                                                                                                                                                                                                                       | 0.1.1        | **Intentionally adding the 6th entry-style wiring must go red**; report-only in this phase, hard-closed in P9                                                                     |
|                         | 0.2.3 `check-boundary.py` (prohibition 2 criterion C)                                                                                                                                                                                                                                                                                                | 0.1.1        | **Intentionally adding one L2→L3 reverse `use` must go red**; record initial `pub(crate)` leakage count                                                                           |
| 0.3 Gate rollout        | 0.3.1 `tools/code-tables` expanded to cover opcode / type tables                                                                                                                                                                                                                                                                                     | 0.1.1        | Depends on P6's T1; closed separately                                                                                                                                             |

> **No size baseline task**. 2026-10-03 decision cancels all line count / volume gates; size issues
> are resolved through responsibility separation (see `08` prohibition 2 criterion A, manual
> judgment). **0.2.1 is the core acceptance of this phase**: if the gate cannot report the known
> parallel representations on unchanged code, the criterion design is wrong — a false gate.

### P1 Resurrect Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Notes S1 + [05](05-frontend-paradigm.md) §Test
Reconstruction

- [x] **1.1 Resurrect `lexer/tests/`** (4 level-3 tasks, completed 2026-10-06: 55 lexical tests
      online; 7 shells deleted; `#[path]` bypass merged into normal wiring to avoid duplicate runs)
- [x] **1.2 Resurrect 4 locations in live directories** (completed 2026-10-06: precedence_inline 6 +
      overload_inline 7 + json 3 + template 7; also deleted 5 lines of residue in
      typecheck/tests/semantic_db.rs)
- [x] **1.3 Fix exposed defects** (completed 2026-10-06, **actual differs from prediction**:
      overflow path and \x/\u escapes are fine in current code, 20 probe tests added for
      never-before coverage; the real exposed defect is "illegal alphanumeric adjacency after radix
      literal is silently split" — `0b102` compiles and runs as wrong value 2, `0o128` as 10,
      `0x1FG`/`123abc` give misleading E1001. Fix: `literals.rs` adds shared helper
      `reject_trailing_alnum` (prohibition 3 compliant: one implementation, five call sites), 5
      red-then-green tests all pass)

| Level 2                                       | Level 3                                                                                                                                                    | File:Line                  | Prerequisite | Acceptance                                                                 |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | -------------------------------------------------------------------------- |
| 1.1 Resurrect `lexer/tests/`                  | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                        | after `lexer/mod.rs:106`   | —            | Test count rises                                                           |
|                                               | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                                | `lexer/tests/mod.rs:15-25` | 1.1.1        | **Without this the 159 lines still don't run**                             |
|                                               | 1.1.3 Delete 7 shell files + their `mod` declarations                                                                                                      | 7 files in `lexer/tests/`  | 1.1.1        | C6 no criterion                                                            |
|                                               | 1.1.4 Delete `pub use ...::*;` re-export block                                                                                                             | `tests/mod.rs:28-38`       | 1.1.3        | This block is exactly what masked the shells                               |
| 1.2 Resurrect 4 locations in live directories | 1.2.1–1.2.4 Connect `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` respectively | each parent module         | —            | Test count rises; **1.2.1 conflicts with 8.0.1, see conflict register C1** |
| 1.3 Fix exposed defects                       | 1.3.1 Four radix scanner overflow paths                                                                                                                    | `literals.rs`              | 1.1.x        | `cargo test` green                                                         |
|                                               | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                       | `literals.rs`              | 1.1.x        | Same                                                                       |

**Acceptance**: test count rises; **expected to expose real defects**, time for fixing must be
reserved. Revert file by file, but **fixed defects should not be rolled back**.

### P2 Establish Equivalence Oracle Baseline

Source: [07](07-equivalence-oracle.md) entire (this document has no "Implementation Notes" section,
reverse-engineered from structure)

- [x] **2.1 IR static verifier** (3 level-3 tasks, all completed 2026-10-06: verify.rs dual mode + 7
      invariants + corpus runs green, actually fixed 3 defects in ir_gen)
- [x] **2.2 Normalized snapshots** (2 level-3 tasks, completed 2026-10-07: normalize.rs + 204
      snapshots in repo + gate 9.5 into CI)
- [x] **2.3 Corpus diffing** (3 level-3 tasks, all completed 2026-10-07)
- [x] **2.4 Vulnerability-specific oracles** (3 level-3 tasks, all completed 2026-10-06: 2.4.1/2.4.3
      turned green with P3, 2.4.4 red-state in place awaiting P4/D20)
- [ ] **2.5 Regression gates** (1 level-3 task)

| Level 2                            | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Prerequisite | Acceptance                                                                         |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------- |
| 2.1 IR static verifier             | 2.1.1 `verify.rs` dual mode `verify_loose` / `verify_ssa` (**completed 2026-10-06**: `src/middle/core/verify.rs`, 07 §102 contract form `verify(ir, mode) -> Result<(), VerifyError>`, violations all collected rather than first-error stop; def/use extraction exhaustive match without wildcard arm, Phi variant introduction causes compile failure to force supplementation)                                                                                                                                                                                                                                          | —            | C4                                                                                 |
|                                    | 2.1.2 **`verify_loose` runs green on existing non-SSA IR** (**completed 2026-10-06**: `test_verify_loose_corpus_green` full corpus 204 files/536 functions/11990 instructions green, built-in non-empty-check assertion on check surface; actually fixed 3 ir_gen defects per D38, corpus diff baseline self-compare zero diff proves behavioral equivalence)                                                                                                                                                                                                                                                              | 2.1.1        | **Hard threshold, prerequisite for P7 batch a** — **unlocked**                     |
|                                    | 2.1.3 Implement 7 invariants (dominance / unique definition / Phi consistency / jump targets / global out-of-bounds / type consistency / inner isolation) (**completed 2026-10-06**: check surface narrowed per actual measurement registration — dominance = named slot must-defined (temp slot Void pre-initialization is frame semantics, ir_gen intentionally relies on), type consistency = parameter slot signature + constant trusted source (ir_gen doesn't write slot type table, full surface goes to P7 value table 04 §262), Phi waits for batch b variant introduction; see verify.rs file header and 06 §G6) | 2.1.1        | C4                                                                                 |
| 2.2 Normalized snapshots           | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / sort predecessors) (**completed 2026-10-07**: `src/middle/core/normalize.rs`, 07's five rules implemented one by one; actually fixed two places — DefId relativized by first-appearance order (intern order unstable across processes, measured 141/143), Arg independent arg% prefix (sharing pool with Local loses variant distinction); instruction printing exhaustive match)                                                                                                                                                           | —            | C1/C3/C5                                                                           |
|                                    | 2.2.2 Snapshots in repo + manual review process (`src/middle/core/tests/snapshots/`) (**completed 2026-10-07**: 204 snapshots mirrored into repo by corpus relative path; update process `UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored` + git diff review + same PR check-in, see tests/snapshot.rs file header; tampering measured to report red precisely)                                                                                                                                                                                                                                                  | 2.2.1        | C1/C3/C5                                                                           |
| 2.3 Corpus diffing                 | 2.3.1 Diff framework + 293 corpus baseline (**completed 2026-10-06**: probe `examples/corpus_probe.rs` + baseline `tests/baselines/corpus-parity.jsonl` (322 entries) + gate `check-corpus-parity.py` into CI (hard gate); normalization includes pointer form scrub, two full runs self-compare zero diff)                                                                                                                                                                                                                                                                                                                | —            | Diagnostics / exit code / stdout item by item                                      |
|                                    | 2.3.2 **Multi-file corpus layer** (create `tests/yaoxiang-multifile/`, with `yaoxiang.toml` project fixture — decision D48) (**completed 2026-10-06**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | —            | **Unconditional must-do (D40), the only executable behavior oracle source for P4** |
|                                    | 2.3.3 C4 behavior diff mandatory coverage list (10 semantic categories) + performance baseline (criterion smoke benchmark) (**completed 2026-10-07**: 10 semantic categories verified full coverage item by item, 26 representative files 26/26 within corpus-parity.jsonl diff baseline; smoke benchmark `benches/pipeline.rs` first test — full corpus compilation 17.6s / CLI cold start 169.7ms, interpreter throughput covered by benches/lib.rs hotpath group)                                                                                                                                                       | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                             |
| 2.4 Vulnerability-specific oracles | 2.4.1 `test_multifile_proof_obligation_not_dropped` (**completed 2026-10-06: red state in place**, measured single file ["E4018"] vs compile_project [])                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | 2.3.2        | **Must be red first**                                                              |
|                                    | 2.4.3 `test_no_silent_pass_on_unproven` (**completed 2026-10-06: red state in place**, trigger source measured to silently pass under compile_project)                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | —            | Intercepts `checker.rs:5179/5318/5448`                                             |
|                                    | 2.4.4 `test_release_plan_spans_consumed` (**completed 2026-10-06: red state in place**, measured 168 files 373 keys not consumed — D20 contract defect empirical evidence, contract fix goes to P4/D20)                                                                                                                                                                                                                                                                                                                                                                                                                    | —            | **No whitelist, difference set must be empty (D41)**                               |
| 2.5 Regression gates               | 2.5.1–2.5.4 see P9 unified list                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | 2.1–2.3      | See P9                                                                             |

> **P2 landed (2026-10-06, 2.1)**: verify.rs dual mode + 7 invariants + full corpus green. **D38
> prediction fulfilled** — the green run actually measured and fixed 3 real defects in ir_gen (see
> 06 §G6 for details): method locals table truncated to parameter count (local_count underestimated,
> runtime relies on Frame::set_slot for tolerance), unannotated parameters dropped by filter_map
> (three places, params metadata mismatched with frame layout), match result slot depends on frame's
> implicit Void initialization (now explicitly pre-initialized). The real semantics of jump target
> is flattened instruction index (not 07's text label-keyed), CFG built at instruction level —
> registered in verify.rs file header. **Commit correspondence**: ir_gen fixes = `a9ea199b`; verify
> main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its message to be
> mis-labeled as "docs(rfc): RFC-027a…", content is actually verify.rs + six test files, buried in
> history and cannot be amended, hereby registered).
>
> **P2 narrowed part landed (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleeding
> channel prerequisites unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract uses same `TestFileSpec` as single-file corpus
>   (single point on library side), binary location / subprocess spawning extracted to
>   `tests/common/` and yx_runner migrated in sync (prohibition 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses rfc027 test's nailed
>   `SumUpTo(3, r)` form (return position / call site / binding position) — 02's `Sorted(3)`
>   annotation example would be substituted and evaluated in current code, directly Disproved, not
>   showing Unproven silence, 07's pseudocode `Sorted(3)` is for illustration only. Red state
>   measurement data in test file header.
> - Two red oracles hang `#[ignore]` resident as 2.4.5 red skeleton precedent: default suite all
>   green, `-- --ignored` reproduces red state; **P3 fix landing removes attribute to turn green**.
> - Rationale for deliberate deviation from 07's (code, file, line) table in comparison
>   normalization (remove file/line, only compare error code set) is in 2.4.1 test comment.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **removed from P2**: the `Program` / `Obligations` types they reference only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (don't double-register), the latter's `#[ignore]` red
> skeleton is merged into 4.3.1.
>
> **2.3.3 landed (2026-10-07)**: C4 mandatory coverage list verified item by item (07 §143), 10
> semantic categories fully covered at corpus level and all within corpus-parity.jsonl diff baseline
> (26 representative files 26/26 hits, grep empirical) — ref/borrow/move full 05-ownership directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure capture
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterators for spawn_for and 16 files, sum types sum_type_* +
> pattern_or_guard + match_call_scrutinee, existential enforcement point interface_dynamic_dispatch
> (Vec(Animal) existential heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic diff
> surface), refinement constraints Drop sequence refined_* ×5 + 05-ownership driven ReleasePlan
> (span key contract has additional 2.4.4 red oracle resident). Performance baseline
> `benches/pipeline.rs` in repo (07 §145 three smokes of two): `corpus_compile_frontend` full corpus
> compilation first test 17.6s, `cli_cold_start_hello` process cold start 169.7ms; interpreter
> throughput covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh goes
> to P9 unified list (2.5).

### P3 Fix Correctness Vulnerabilities (Minimal Solution)

Source: [02](02-stage-contract.md) §Implementation Notes S3 vulnerability fix half + **2026-10-07
silent drop point audit (3.4)**

- [x] **3.1 `proof_calls` consumer side** (2 level-3 tasks, completed 2026-10-06)
- [x] **3.2 Second silent drop point** (1 level-3 task, completed 2026-10-06; **description
      mismatches actual measurement, see 3.2.1 row correction**)
- [x] **3.3 panic to diagnostic** (1 level-3 task, completed 2026-10-06)
- [ ] **3.4 Silent drop point addendum** (8 level-3 tasks, 2026-10-07 audit newly added;
      3.4.1/3.4.2/3.4.3 implemented, 3.4.4–3.4.8 advance with P4 prerequisites)

| Level 2                                                       | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                    | File:Line                                                                                    | Prerequisite | Acceptance                                                                                          |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------- |
| 3.1 `proof_calls` consumer side                               | 3.1.1 `proof_calls` narrow visibility + sole getter (**2026-10-07 verification: actually `pub(super)` + `pub fn proof_calls()`, not "private"**)                                                                                                                                                                                                                                                                                                           | `types.rs:36/84`                                                                             | 2.4.1        | Vulnerability oracle turns green                                                                    |
|                                                               | 3.1.2 Consumer points completed                                                                                                                                                                                                                                                                                                                                                                                                                            | `orchestrator.rs:99` / `:273` / `:450` / `:1374` (measured 4 entries + single file 5 places) | 3.1.1        | C2                                                                                                  |
| 3.2 Second silent drop point                                  | 3.2.1 `Unproven` empty match arm produces diagnostic (**2026-10-07 verification: `ownership.rs` only constructs `Proved`/`Disproved`, this arm source unreachable, defensive code; oracle turns green carried by 3.1, this item has no independent oracle**)                                                                                                                                                                                               | `checker.rs:1303-1314`                                                                       | 2.4.3        | `test_no_silent_pass_on_unproven`                                                                   |
| 3.3 panic to diagnostic                                       | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic                                                                                                                                                                                                                                                                                                                                                                                                      | `predicate.rs:34-36`                                                                         | —            | No longer panic when Z3 missing                                                                     |
| 3.4 Silent drop point addendum (2026-10-07 audit newly added) | 3.4.1 const generic constraint `Unproven` empty arm changed to produce diagnostic/accounting (**audit newly added: zero registration in entire warehouse documentation, zero tasks**) (**implemented 2026-10-07**: wiring has registered W1063 — fact chain environment→ExpressionInferrer→StatementChecker→checker convergence emission; bounds layer distinguishes TypeVar internal probe actual arguments to prevent false positives; red oracle first) | `environment.rs:422-426`                                                                     | —            | New oracle red-then-green: const actual argument must not be silent when not compile-time evaluable |
|                                                               | 3.4.2 Termination check solver missing trace (**implemented 2026-10-07**: new code W1081 — `count_unjudged_obligations` pure function takes signal (decreasing + well-foundedness two tables merged), checker convergence emission; NotProved (solver present but cannot judge) does not trigger, this code is exclusively for "solver missing")                                                                                                           | `checker.rs:1266-1268`                                                                       | —            | W-level diagnostic when Z3 missing; same degradation philosophy as 3.3.1                            |
|                                                               | 3.4.3 02 §91 phase coverage table row 1 sync (**completed 2026-10-07**: table kept as 9e02e4db diagnostic snapshot, review notes cell-by-cell marked current destination; `#434` decision additionally registered as RFC-039 **D57**)                                                                                                                                                                                                                      | `02:95-107`                                                                                  | —            | Manual review: each cell cross-checked with code                                                    |
|                                                               | 3.4.4 `variant_ctor_calls` zero consumer decision (delete field or wire consumer; IR side currently uses form detection `detect_variant_ctor_call`)                                                                                                                                                                                                                                                                                                        | `types.rs:56`, `ir_gen.rs:7374`                                                              | 4.3.1        | `check-obligations.py` (9.2) no longer reports this field; no silent ambiguous form                 |
|                                                               | 3.4.5 `method_overload_ir_names` and `overload_resolutions` parallel representation choose one (prohibition 1)                                                                                                                                                                                                                                                                                                                                             | `types.rs:60`, `ir_gen.rs:369`                                                               | 4.3.1        | Same fact single point expression; C3                                                               |
|                                                               | 3.4.6 `compile_project` add `warnings` consumption (multi-file `run` never reports W1001/W1002/W1003)                                                                                                                                                                                                                                                                                                                                                      | `orchestrator.rs:99-159`                                                                     | 4.2.1        | C2: multi-file and single-file diagnostic sets unified                                              |
|                                                               | 3.4.7 LSP non-project-internal path access unified consumer (currently directly calls `check_module_collect_all`: doesn't run proof, doesn't read warnings, doesn't do dead code analysis)                                                                                                                                                                                                                                                                 | `lsp/handlers/diagnostics.rs:161-206`                                                        | 4.2.6        | LSP and CLI same diagnostic set for same file                                                       |
|                                                               | 3.4.8 Multi-file monomorphization arm (`instantiation_requests` sole consumer is in single-file pipeline; cross-file generic matrix probe measured passing, is **potential** risk unproven defect)                                                                                                                                                                                                                                                         | `orchestrator.rs:174`, `pipeline.rs:330`                                                     | 4.1.3        | C2: cross-file generic matrix equivalent to single-file                                             |

**Acceptance**: vulnerability oracles turn green, and **intentionally removing the fix must
re-trigger red**. **Risk**: adding E4018 in multi-file path is a breaking change.

**3.4 acceptance**: 3.4.1 / 3.4.2 must have red oracles first (no code changes before red); 3.4.3 is
manual review; 3.4.4–3.4.8 use C2/C3 and P9's 9.2 gate. **Risk**: 3.4.1 new diagnostic is a behavior
change, must sync corpus diff baseline and CHANGELOG.

> **P3 landed (2026-10-06)**: 3.1.1 `proof_calls` narrow visibility + sole getter (then recorded as
> `pub(crate)`; **2026-10-07 verification actual landing is `pub(super)`** — code comment references
> `#434` decision with no documentation registration, see 3.4.3; fully private requires rewriting 11
> construction literals, not taken in stop-bleeding phase; P4 Obligations migration will close);
> execution mechanism extracted to `src/frontend/proof_execution.rs` (prohibition 3 single point —
> cannot put in typecheck/, that would create L3→L4 reverse dependency); 3.1.2 completed **four
> entries** per 02 evidence chain step 5 (WBS row missed `compile_project :99`, necessary to
> complete and register per 3.1.1 acceptance "oracle turns green"); 3.2.1 empty arm changed to
> accounting (throw on `proof_calls`) + diagnostic (`into_result` established conversion path);
> 3.3.1 SOLVER slot made Optional, Z3 missing degrades conservatively to `SMTResult::Unknown` (E2031
> family diagnostic replaces process crash). Two vulnerability oracles removed `#[ignore]` to normal
> green, added corpus fixture `proof-obligation-honored`; check-fanout inventory migrated
> accordingly (wiring point 19 = inventory). Measurement: oracles 2.4.1/2.4.3 turned green, full
> suite lib 2494 / integration 272 / corpus dual layer all green.
>
> **3.4 audit (2026-10-07, P3 post-landing review)**: For defects of the category "failure produces
> no signal" (definition see 02 §41-52), re-inventoried on unchanged code, located 8 more, all
> registered as 3.4.x in the above table. Method and evidence:
>
> - **Consumer surface inventory** (16 obligation fields grep read points one by one):
>   `variant_ctor_calls` **zero consumer** — sole read `checker.rs:1468` is "self-injected into
>   `TypeCheckResult`", IR side uses form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688` only
>   asserts producer side, exactly the "only test producer side, immune to consumer side defects"
>   form criticized in 02 §186 → 3.4.4. `method_overload_ir_names` result-level field also zero
>   external consumer, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Phase coverage review**: 02 §91 table row 1 (four entries "no proof_execution") already fixed
>   by 3.1.2, current state is 5 places sharing one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 current state unchanged (multi-file `run`
>   still doesn't output W1003, still doesn't run dead code family) → 3.4.6; LSP
>   non-project-internal path (02 §77 "manual lex→parse") also doesn't run proof → 3.4.7.
> - **Same-form empty arm scan** (`Unproven` has only four consumers in entire warehouse): checker
>   three refinement branches (3.1 covered), 3.2.1 fixed ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostic, zero accounting, and zero registration in
>   entire warehouse docs**) → 3.4.1, is this audit's only "all-unknown" item.
> - **Solver acquisition surface**: `default_solver()` three consumer forms vary (process singleton
>   at `predicate.rs:32` / per `check_module` injection at `checker.rs:1266` / new per back-edge
>   judgment at `ownership.rs:621`) — 3.3.1 only eliminated one panic; other two's **silent
>   surface** goes to 3.4.2, **singleton and cache hit** goes to P4's 4.4.2 (don't double-register).
> - **Not counted into 3.4 with existing ownership**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys unconsumed, Drop silently lost) still P2 red resident, ownership 4.5.1/D20
>   unchanged; 4.4.1 (layer order declaration opposite to actual, `equivalence` layer not in
>   pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already registered in P4.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisite, can land on-site at any
> time (suggest doing 3.4.1 first — the only "all-unknown" live vulnerability, smallest change
> surface); 3.4.4–3.4.8 advance with P4 execution, their oracles depend on obligation ledger (4.3.1)
> or unified Driver (4.2.x). **3.4 does not establish new oracle system**, uniformly reuses existing
> C2/C3 and P9's 9.2 gate.
>
> **Stop-bleeding channel (2026-10-05 decision)**: P3 doesn't need to wait for P2 to complete.
> 3.1/3.2's only hard prerequisites are **2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red
> oracles)**; 2.1/2.2/2.3.1/2.3.3 (IR verifier, snapshots, single-file diff, performance baseline)
> can advance in parallel with P3. Correctness vulnerability (`Sorted(3)` silently passing)
> stop-bleeding should not be blocked by snapshot infrastructure construction. 3.3 (panic to
> diagnostic) has no prerequisite, can land at any time.

### P3.5 RFC-029g: Remove `pub` and Auto-Binding (External Track / #399 — Ordering Constraint D56)

Source: [RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md) (accepted 2026-10-02,
implementation not started), [#399](https://github.com/ChenXu233/YaoXiang/issues/399). **This track
is not a refactoring design item itself, but a spec compliance removal forcibly pre-positioned by
ordering constraints**; it is two different things from P10 S5 "narrow `pub` item visibility" (Rust
`pub(crate)` narrowing).

**Why it must be entirely before P4**: 029g's removal surface overlaps with the rewrite surface of
four refactoring phases, reverse order = redoing the same batch of removals and baseline updates on
the new grammar / new checker layout / new SSA IR.

| 029g Removal Surface                                               | File                                                                                                               | Rewritten by Which Phase                                                                                                                |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `KwPub` and grammar branch                                         | `lexer/{tokens,state,mod}.rs`; `parser/parser_state.rs`, `statements/{declarations,imports,functions,bindings}.rs` | **P8** (8.2 lexical convergence / 8.3 build grammar / 8.6 delete Pratt / 8.7 split `parse_assign_after_target`)                         |
| AST `is_pub` ×2 (`Assign` / `TypeDefinition`)                      | `parser/ast.rs:247/270`                                                                                            | **P6 6.5 AST dead variant** + P8 + D1 directory rename (`parser/ast.rs` → top-level `src/ast/`)                                         |
| `auto_bind_to_type` and `collect_exports` pub branch               | `typecheck/checker.rs:2150/3802/3836`, `environment.rs:284`                                                        | **P5** (`include!`→real `mod` and checker split is **pure relocation**, pub branch will be moved as-is into new file)                   |
| Dead code exemption `exempt_pub` / `is_exported` and role dispatch | `typecheck/passes/dead_code.rs`, `module/orchestrator.rs:389-407` (within `check_project`)                         | **P4 4.2.2** (`check_project` slim down)                                                                                                |
| `semantic_tokens` `Public` modifier                                | `typecheck/checker/semantic_tokens.rs:450/498`                                                                     | **P4 4.2.6** + P5 + 4.6 (D55)                                                                                                           |
| `ir_gen` `is_pub` ignore binding                                   | `middle/core/ir_gen.rs:1378`                                                                                       | **P7** (`ir_gen.rs` SSA-ification)                                                                                                      |
| `src/std/` 31 `pub`                                                | `std/list.yx`(25) / `json.yx`(4) / `option.yx`(1) / `result.yx`(1)                                                 | No phase directly modifies, but **P6 hard acceptance requires `git diff --stat tests/ src/std/` to be empty** — can only land before P6 |

**Incidental benefit**: 029g deletes `env.exports` dead table (producer injects, zero consumer in
entire warehouse) and `is_exported`/`is_visible` two accessors → P4's 4.3 obligation ledger loses
one "produced but unconsumed" field.

- [ ] **3.5.1 Lexical and parser branch** (`KwPub` ×8)
- [ ] **3.5.2 AST `is_pub` ×2** (including production construction sites and test fixture sync)
- [ ] **3.5.3 Checker side** (`auto_bind_to_type`, `collect_exports` pub branch, `env.exports` dead
      table and two accessors)
- [ ] **3.5.4 Dead code exemption convergence** (delete `exempt_pub` / `is_exported`; `orchestrator`
      role dispatch sync)
- [ ] **3.5.5 Peripheral echo** (formatter pub prefix, `semantic_tokens` `Public`)
- [ ] **3.5.6 std and diagnostic text** (31 pub prefixes; 4 W1xxx help "pub is external interface"
      statement change zh source, translation via bot)
- [ ] **3.5.7 Language reference convergence** (`modules.md` §3.2–3.3, `syntax.md` keyword table
      18→17, `warning-codes.md`, `language-overview.md`, `guide/modules.md`)

| Level 2                   | Level 3                                                                                                        | File:Line                                                                                                            | Acceptance                                                                                                                                 |
| ------------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| 3.5.1 Lexical and parser  | 3.5.1.1 `KwPub` variant and `keyword_from_str` entry deletion                                                  | `lexer/tokens.rs:83`, `state.rs:28`, `mod.rs:62`                                                                     | Keyword table 18→17 (`Kw*` 16→15, including `tokens.rs:82` count comment)                                                                  |
|                           | 3.5.1.2 Declaration-position pub detection and `pre_detected_pub` deletion                                     | `parser/statements/declarations.rs:672/675/731/747/880`, `functions.rs`, `bindings.rs`                               | `pub f = ...` reports parse error (**red oracle first**)                                                                                   |
|                           | 3.5.1.3 use brace item pub skip and statement leading pub deletion                                             | `parser/statements/imports.rs:57`, `parser_state.rs:203`                                                             | `pub use m` reports parse error (**red oracle first**)                                                                                     |
| 3.5.2 AST                 | 3.5.2.1 `Assign`/`TypeDefinition` `is_pub` deletion + construction site sync                                   | `parser/ast.rs:247/270` and each construction site                                                                   | No pub residue in compiler (`grep -rn is_pub src/` empty)                                                                                  |
| 3.5.3 Checker             | 3.5.3.1 `auto_bind_to_type` deletion (pub is the only call reason)                                             | `checker.rs:3802`, `environment.rs:284`, call site `checker.rs:2150`                                                 | Method form only explicit combination remains (RFC-004), std and corpus zero dependency                                                    |
|                           | 3.5.3.2 `collect_exports` remove pub branch + `env.exports` dead table and `is_exported`/`is_visible` deletion | `checker.rs:3836`, `environment.rs:599/612`                                                                          | Export surface sole authority = module registry `ModuleInfo.exports`; 9.2 gate one less field                                              |
| 3.5.4 Dead code exemption | 3.5.4.1 `exempt_pub` / `is_exported` and exemption branch deletion                                             | `dead_code.rs:23/44/83/90/174/180/191/195/591/629`                                                                   | Lib/Script unreferenced top-level binding reports W1001 (**red oracle first**)                                                             |
|                           | 3.5.4.2 `orchestrator` role dispatch sync                                                                      | `orchestrator.rs:389-407`                                                                                            | Single-file / multi-file diagnostic set unified (C2), W code doesn't mix into consumer                                                     |
| 3.5.5 Peripheral echo     | 3.5.5.1 formatter pub prefix                                                                                   | `formatter/handlers/stmt.rs:81/117/146/151`                                                                          | format idempotent (existing oracle)                                                                                                        |
|                           | 3.5.5.2 `semantic_tokens` `Public` modifier                                                                    | `typecheck/checker/semantic_tokens.rs:450/498`                                                                       | Semantic coloring no longer produces Public                                                                                                |
| 3.5.6 std and text        | 3.5.6.1 `src/std` 31 pub prefix deletion                                                                       | `std/{list,json,option,result}.yx`                                                                                   | Corpus and std self-check all green; **must complete before P6**                                                                           |
|                           | 3.5.6.2 Diagnostic text                                                                                        | 4 W1xxx help                                                                                                         | No longer claims "pub is external interface"; aligned with `dead_code.rs:574` Bin role behavior                                            |
| 3.5.7 Documentation       | 3.5.7.1 Language reference and guide                                                                           | `reference/language-spec/{modules,syntax,language-overview}.md`, `warning-code/warning-codes.md`, `guide/modules.md` | Spec no longer has "pub still writable" statement opposite to implementation; `warning-codes.md:124` "pub never triggers W1001" also fixed |

**Acceptance (behavior change, red oracle first)**:

1. **Red-then-green**: two behavior changes each land red oracle before modifying code — ① `pub`
   changes from "silently accepted" to parse error; ② Lib/Script role unreferenced top-level binding
   newly reports W1001 (previously pub items absolutely exempted).
2. **Test count must not decrease**: when deleting `typecheck/passes/tests/dead_code.rs`
   `exempt_pub` cases and `is_pub` fixtures, must be replaced by equivalent new cases.
3. **Baseline same commit**: diagnostic set changes (new parse error + new W1001) → corpus diff
   baseline (`tests/baselines/corpus-parity.jsonl`) and CHANGELOG updated in same commit; IR
   snapshot expected zero-diff, if not zero must explain reason.
4. **Independent revert unit**: whole batch occupies one commit group, not mixed with other phases
   (DoD 6).
5. `cargo test` / `clippy -D warnings` / `python scripts/rfc/check_tracking.py` all green.

**Ordering**: overlaps with 3.4 files (`checker.rs`, `environment.rs`) → the two must be serial, not
parallel; **entirely before P4** (RFC-039 D56). P3.5 doesn't depend on 3.4, which to do first is
determined by implementer per on-hand context.

### P4 Phase Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Notes S1–S5. **This phase and P3's fix must be
split into two commits.**

- [ ] **4.1 Declarative phase table** (5 level-3 tasks)
- [ ] **4.2 Entry merging** (8 level-3 tasks)
- [ ] **4.3 Obligation ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm wrap-up** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ization (D20)** (2 level-3 tasks)
- [ ] **4.6 LSP semantic data pipeline unification (D55)** (1 level-3 task)

| Level 2                                       | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Prerequisite | Acceptance                                                                                                                                                |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative phase table                   | 4.1.1 `stage.rs` (`Stage` variant + `Stage::ALL` + `StageScope`) (**completed 2026-10-07**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | P0           | `test_program_stage_coverage`                                                                                                                             |
|                                               | 4.1.2 `program.rs` / `unit.rs` (**completed 2026-10-07**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 4.1.1        | Same                                                                                                                                                      |
|                                               | 4.1.3 `Pipeline::run` rerouted to Driver (`pipeline.rs:141-227`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 4.1.2        | **Single-file diagnostic set and exit code byte-identical**                                                                                               |
|                                               | 4.1.4 Phase coverage assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 4.1.3        | C2                                                                                                                                                        |
|                                               | 4.1.5 Full corpus zero-diff                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | 4.1.4        | Diagnostic / exit code / stdout all zero-diff (293 corpus)                                                                                                |
| 4.2 Entry merging                             | 4.2.1 `compile_project` slimmed to Program constructor (`orchestrator.rs:99-237`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 4.1.5        | Four entries diagnostic set identical after unification                                                                                                   |
|                                               | 4.2.2 `check_project` (`:273-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | 4.2.1        | Same                                                                                                                                                      |
|                                               | 4.2.3 `check_source_in_project` (`:450`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | 4.2.2        | Same                                                                                                                                                      |
|                                               | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 4.2.3        | Same                                                                                                                                                      |
|                                               | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 4.2.1        | `check` consistent inside and outside project                                                                                                             |
|                                               | 4.2.6 Delete LSP manual phase sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 4.2.1        | LSP and CLI same diagnostic set                                                                                                                           |
|                                               | 4.2.7 `Aggregation` parameter-driven two-choice (`orchestrator.rs:486-494`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | 4.2.1        | **Line-by-line verify the two function internal differences during implementation** (conflict register C5 has been adjudicated as mandatory verification) |
|                                               | 4.2.8 wasm reroutes via `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | 4.2.1        | See C6                                                                                                                                                    |
| 4.3 Obligation ledger                         | 4.3.1 `obligations.rs` + `assert_drained()` (new; includes `#[ignore]` red skeleton first)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | 4.1.3        | `test_obligations_drained` turns green                                                                                                                    |
|                                               | 4.3.2 Obligation diagnostic W→E upgrade                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | 4.3.1        | Manual review each new E                                                                                                                                  |
| 4.4 Proof layer and wasm wrap-up              | 4.4.1 `layers/README.md` layer order changed to actual order                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | —            | **Walks independently of 4.1–4.3**                                                                                                                        |
|                                               | 4.4.2 SMT solver acquisition surface unified to process-level shared singleton (**per D58**: actual fix location `backend.rs` — `LazyLock<Mutex<Option<Box<dyn Solver>>>>` + `with_shared_solver` closure port; `predicate.rs` delete private SOLVER, `checker.rs` termination injection changed to in-closure judgment (`TerminationChecker` lifetime parameterized), `ownership.rs` back-edge judgment changed to shared port; `default_solver()` kept for testing only. **Hard prerequisite**: Unknown not in cache (otherwise timeout results solidify across compilations/tests via process-level cache). 89576fafd only delivered counter observation tool, this task body not touched) | —            | Production path `default_solver()` call sites go to zero + cache hit rate observable across three consumer points                                         |
|                                               | 4.4.3 Add warning to degradation path (`checker.rs:1283-1293`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | —            | —                                                                                                                                                         |
| 4.5 Cross-layer contract PlanId-ization (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (`layers/ownership.rs:31` producer-side allocation + `ir_gen.rs:1942` consumer-side matching)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | 4.3.1        | `test_release_plan_spans_consumed` difference set empty (D41)                                                                                             |
|                                               | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | 4.5.1        | C2; span mismatch silent failure goes to zero                                                                                                             |
| 4.6 LSP semantic data pipeline unification    | 4.6.1 Project-internal file semantic data (SemanticDB) routes through orchestrator same pipeline, co-sourced with diagnostics; cross-file references carry `resolves_to`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | 4.2.1        | Project-internal cross-file jump hits definition; semantic data and diagnostics from same compilation                                                     |

### P5 checker File-Internal Split — Filled in by This Document (Original Gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (`collect_used_in_type`'s
`pub(crate)` path must not change)

- [ ] **5.1 `include!` → real `mod`** (2 level-3 tasks)
- [ ] **5.2 Split out `refinement`** (2 level-3 tasks)
- [ ] **5.3 Split out `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                                          | Prerequisite | Acceptance                                               |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------- |
| 5.1 `include!` → real `mod` | 5.1.1 `checker.rs:5618` `include!` changed to real `mod` declaration                                                                                                                                                                                             | P4 complete  | C1 zero-diff; `semantic_tokens.rs` gains module identity |
|                             | 5.1.2 Add `use super::*` or item-by-item introduction (implicit scope inheritance in `include!` era)                                                                                                                                                             | 5.1.1        | `cargo build` green                                      |
| 5.2 Split out `refinement`  | 5.2.1 Migrate refinement block (currently `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                                   | 5.1.1        | C1 zero-diff                                             |
|                             | 5.2.2 `collect_refined_binding_checks` signature refactor: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                           | 5.2.1        | C1; call site only `1235`                                |
| 5.3 Split out `annotations` | 5.3.1 Migrate annotation validation (currently `3441-3661`, including `is_predicate_head`)                                                                                                                                                                       | 5.2.2        | C1 zero-diff                                             |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) keeps `pub(crate)` and stays under `checker` module path** → add `pub(crate) use annotations::*;` at top of `checker.rs`, making `inference/statements.rs:16` import **unchanged character-for-character** | 5.3.1        | **Zero caller changes** is this step's hard acceptance   |

**Hard constraint**: `collect_used_in_type` is used cross-module by `inference/statements.rs:16`.
Method is `pub(crate) use` re-export, making the split **pure relocation**.

**Only split confirmed large blocks.** Don't split modules of only 200–400 lines — increases
navigation cost, reduces cohesion, zero decoupling benefit. Other submodules (signatures / type_defs
/ imports) wait for file to grow back to 1,500 lines before splitting.

### P6 Type Representation Unification

Source: [03](03-type-unification.md) §Implementation Notes stages 0–6, criterion C3. **Stages 0→6
strictly serial.**

- [ ] **6.0 Gate first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead variant deletion** (2 level-3 tasks)
- [ ] **6.4 parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variants** (2 level-3 tasks)
- [ ] **6.6 Gate hardens** (1 level-3 task)
- [ ] **6.7 Directory rename (D1, pure relocation batch)** (1 level-3 task)
- [ ] **6.8 Annotation check mode and Fn parameter representation (D54, 2026-10-05 added)** (2
      level-3 tasks)

| Level 2                                                         | Level 3                                                                                                                                                                                                                                                                                                                                                                       | File:Line                                                             | Acceptance                                                                                                                                                                                                  |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.0 Gate first                                                  | 6.0.1 Create `tools/type-tables`, implement T1/T2/T3, **T1 first report-only not failing**                                                                                                                                                                                                                                                                                    | New crate                                                             | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge reconstruction points listed separately**                                                                                      |
|                                                                 | 6.0.2 Take full baseline on unchanged code                                                                                                                                                                                                                                                                                                                                    | —                                                                     | No code change                                                                                                                                                                                              |
| 6.1 Name normalization                                          | 6.1.1 Count non-canonical type name occurrences in corpus                                                                                                                                                                                                                                                                                                                     | `tests/yaoxiang/`                                                     | Decide whether synonyms can be deleted directly                                                                                                                                                             |
|                                                                 | 6.1.2 `from_builtin_name` cut synonyms → lexer alias table                                                                                                                                                                                                                                                                                                                    | `mono.rs:618-643`                                                     | C3                                                                                                                                                                                                          |
|                                                                 | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                                                                                                                                                                                          | `types.rs:162-165`                                                    | **Must verify line by line, cannot assume equivalence**                                                                                                                                                     |
| 6.2 Bytecode type unification                                   | 6.2.1 Three places changed to `MonoType`                                                                                                                                                                                                                                                                                                                                      | `bytecode.rs:812/814/854`, `image.rs:44`                              | C3 + `dump_bytecode` targeted compare                                                                                                                                                                       |
|                                                                 | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                                                                                                                                                                                      | `bytecode.rs:2352-2390`                                               | Same                                                                                                                                                                                                        |
|                                                                 | 6.2.3 `type_table` element type changed to `MonoType`                                                                                                                                                                                                                                                                                                                         | `image.rs:44`                                                         | Same                                                                                                                                                                                                        |
| 6.3 Dead variant deletion                                       | 6.3.1 Delete 11 variants + clear match arms + fallback                                                                                                                                                                                                                                                                                                                        | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                           | C3 + `cargo build` green                                                                                                                                                                                    |
|                                                                 | 6.3.2 **Must be in same batch as 6.3.1** rewrite reverse bridge                                                                                                                                                                                                                                                                                                               | `passes/mono/function.rs:507-529` (`mono_to_ast_type` reconstructs 7) | Generic substitution targeted compare, otherwise **silently changes behavior**                                                                                                                              |
| 6.4 parser data flow                                            | 6.4.1 New `probe.rs`                                                                                                                                                                                                                                                                                                                                                          | New                                                                   | C3                                                                                                                                                                                                          |
|                                                                 | 6.4.2 Modify `parser_state.rs:46-58`                                                                                                                                                                                                                                                                                                                                          | —                                                                     | Same                                                                                                                                                                                                        |
|                                                                 | 6.4.3 Delete `operator_interfaces::spec()` calls (**2 places**: `declarations.rs:509`, `ast.rs:930`)                                                                                                                                                                                                                                                                          | —                                                                     | Same                                                                                                                                                                                                        |
|                                                                 | 6.4.4 Merge two copies of `name_used_as_type*`                                                                                                                                                                                                                                                                                                                                | `ast.rs:847` + `declarations.rs:42`                                   | Same                                                                                                                                                                                                        |
|                                                                 | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) routed via `NameKind::Builtin`                                                                                                                                                                                                                                                                                                       | —                                                                     | Same                                                                                                                                                                                                        |
| 6.5 AST dead variants                                           | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                                                                                                                                                                                                    | `ast.rs:37-43` + 4 consumer places + 2 exhaustive arms                | `tests/integration/` 18 modules green (**split 2 commits**)                                                                                                                                                 |
|                                                                 | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                                                                                                                                                                                               | `ast.rs:241-249`                                                      | Mark **semantic risk**                                                                                                                                                                                      |
| 6.6 Gate hardens                                                | 6.6.1 T1 changed from warning to `panic!`                                                                                                                                                                                                                                                                                                                                     | `tools/type-tables`                                                   | **Intentionally introducing zero-construction variant must go red**                                                                                                                                         |
| 6.7 Directory rename (D1, pure relocation batch)                | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; `parser/ast.rs` split out to top-level `ast/`; bytecode domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` migrate to `middle/bytecode/`                                                                                                                                                                   | Entire warehouse                                                      | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline doesn't regress                                                                                                                   |
| 6.8 Annotation check mode and Fn parameter representation (D54) | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>` (name and type cohabit), closed in same batch as 6.5.2 deleting `Assign.signature_params` — 6.5.2 only deletes old representation, this item provides replacement representation                                                                                                                                            | `ast.rs:451-454`, consumer surface ~20 files (mechanical change)      | C3 (pure representation reshape batch, compare not enabled); split into independent commit                                                                                                                  |
|                                                                 | 6.8.2 Declaration-driven check mode: parameter type by position drives lambda head (independent of name), exit (tail expression and return) unified against annotation return type, non-lambda value infer+unify (isomorphic with `m: Int = "hello"`); red oracle first — u1–u4/v1–v5 probes into corpus (red-then-green); delete `declarations.rs:337-376` parser name merge | `statements.rs:1170-1914`                                             | Compare-enable batch is **behavior fix**: new diagnostic code, baseline update, C3 not applicable; v1–v5 five syntactic forms same judgment (eliminate "same semantics different syntax one red one green") |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA-ification

Source: [04](04-ssa.md) §Implementation Notes batches a–d, criterion C4, **strictly serial within
batch**.

- [ ] **7a Cut multiple definition** (3 level-3 tasks)
- [ ] **7b SSA form switch** (6 level-3 tasks; 7b.1 split into 1a/1b per DoD 6)
- [ ] **7c Implicit contract explicitation** (3 level-3 tasks)
- [ ] **7d Last** (2 level-3 tasks)
- [ ] **7e `.42` data loss fix** (3 level-3 tasks; 2315/2341 incorporated from "independent issue",
      2026-10-05)

| Level 2                            | Level 3                                                                                                                                      | Prerequisite                        | Acceptance                                    |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | --------------------------------------------- |
| 7a Cut multiple definition         | 7a.1 Delete 3 statement-level recycles                                                                                                       | **2.1.2 `verify_loose` runs green** | C4                                            |
|                                    | 7a.2 6 save/restore replaced with RAII guard                                                                                                 | 7a.1                                | C1 snapshot zero-diff                         |
|                                    | 7a.3 Caliber unification (`1798` vs `2001`)                                                                                                  | 7a.2                                | C1                                            |
| 7b SSA form switch                 | 7b.1a `ir.rs` structural change #1-2 (`Operand` variant convergence + `Instruction::Phi` introduction)                                       | 7a.3 + **P6 complete**              | C4; exclusive commit                          |
|                                    | 7b.1b `ir.rs` structural change #3-5 (`FunctionIR` / `BasicBlock` / type annotation fields)                                                  | 7b.1a                               | C4; exclusive commit                          |
|                                    | 7b.2 `next_temp_reg` changed to `Operand::Value`                                                                                             | 7b.1                                | Same                                          |
|                                    | 7b.3 `translator.rs` add `Phi` arm                                                                                                           | 7b.1                                | C4 + `.42` round-trip test                    |
|                                    | 7b.4 Linear scan allocator (within u8/255 slot model)                                                                                        | 7b.3                                | Same (+500~1000 lines)                        |
| 7c Implicit contract explicitation | 7c.1 `synth.rs` boundary + boundary check script                                                                                             | 7b.4                                | C4 + new script into CI                       |
|                                    | 7c.2 Span consumption count                                                                                                                  | 7c.1                                | Consumption count **zero triggers**           |
|                                    | 7c.3 `method_def_ordinals` read-only                                                                                                         | 7c.1                                | Ownership in question (C7)                    |
| 7d Last                            | 7d.1 `generate_call_expr_ir` split into `CallArgs` + 6 `emit_*`                                                                              | 7c.x                                | C1 snapshot zero-diff                         |
|                                    | 7d.2 "补 0" fallback changed to return diagnostic                                                                                            | 7d.1                                | C4                                            |
| 7e `.42` data loss fix             | 7e.1 `upvalue_count: 0` fix (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + already-allocated unused opcode reclaim (D32/D34 with version bump) | 7d.x                                | C4 + `.42` round-trip test; exclusive commit  |
|                                    | 7e.2 `exception_handlers` write to disk (`bytecode.rs:2315` — throw/try is language core semantics, **must not be left for implementation**) | 7e.1                                | `.42` directly runs exception case diff       |
|                                    | 7e.3 `globals` write to disk (`bytecode.rs:2341`)                                                                                            | 7e.1                                | `.42` directly runs global variable case diff |

**Expectation management**: net **+900–1600 lines**. If the project uses "code gets shorter" as
success criterion at project launch, this phase will be judged a failure.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Notes stages 0–4, criterion C5.

- [ ] **8.0 Dead step cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexical convergence** (2 level-3 tasks)
- [ ] **8.3 Build LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual parser diffing** (1 level-3 task)
- [ ] **8.5 Switch stream** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 `parse_assign_after_target` split** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)
- [ ] **8.9 Parameter position form unification (D53, 2026-10-05 added)** (1 level-3 task)

| Level 2                                       | Level 3                                                                                                                                                                                                                                                                                                             | Prerequisite                     | Acceptance                                                                                                       |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead step cleanup (C6)                    | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                                                                                                                                                                                                                                     | **Conflicts with 1.2.1, see C1** | C6 no criterion                                                                                                  |
|                                               | 8.0.2 Bare magic numbers named (`(6,7)` / `(11,1)` / `12`)                                                                                                                                                                                                                                                          | —                                | Same                                                                                                             |
| 8.2 Lexical convergence                       | 8.2.1 Four radix scanner merge + three escape merges + multi-line string merge                                                                                                                                                                                                                                      | —                                | C5; `literals.rs` reduces ~500 lines                                                                             |
|                                               | 8.2.2 f-string nested compilation elimination                                                                                                                                                                                                                                                                       | 8.2.1                            | **Span must change**, must build baseline first                                                                  |
| 8.3 Build LALRPOP grammar                     | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                                                                                                                                                                                                                                       | 8.2.x                            | `cargo build` passes; can produce AST for 293 corpus (**no comparison**)                                         |
|                                               | 8.3.2 Action code `grammar/actions.rs` (one function per `Expr` variant, 22 in total)                                                                                                                                                                                                                               | 8.3.1                            | Same                                                                                                             |
|                                               | 8.3.3 Error productions (keep `Expr::Error` / `StmtKind::Error` placeholder semantics)                                                                                                                                                                                                                              | 8.3.2                            | Same                                                                                                             |
| 8.4 **Dual parser diffing**                   | 8.4.1 Two parsers each run 293 corpus + `src/std/tests`, AST normalized then **compared bit by bit**                                                                                                                                                                                                                | 8.3.3                            | **Valid program AST all equivalent — this is the equivalence proof for the entire grammar migration**            |
| 8.5 Switch stream                             | 8.5.1 `parse()` changed to call LALRPOP, Pratt kept as `parse_legacy()`                                                                                                                                                                                                                                             | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span item by item identical (**no relaxation**, no C5′ exists) |
| 8.6 Delete Pratt                              | 8.6.1 Delete `nud.rs`(1326) + `led.rs`(451) + two BP step systems + ~89 references                                                                                                                                                                                                                                  | 8.5.1                            | Corpus all green; `git grep BP_` zero hits                                                                       |
|                                               | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally dead)                                                                                                                                                                                                                                                    | 8.6.1                            | Same                                                                                                             |
| 8.7 `parse_assign_after_target` split         | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                                                                                                                                                                                                                                              | 8.6.x                            | Function only does dispatch                                                                                      |
| 8.8 Test completion                           | 8.8.1 New associativity cases (**red-then-green**)                                                                                                                                                                                                                                                                  | 8.2.x                            | C5                                                                                                               |
|                                               | 8.8.2 Complete four literal error paths                                                                                                                                                                                                                                                                             | —                                | Cover untested paths                                                                                             |
|                                               | 8.8.3 Complete `pratt/tests/mod.rs` wiring self-check assertion                                                                                                                                                                                                                                                     | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6, changed to `parser/tests/`**)            |
| 8.9 Parameter position form unification (D53) | 8.9.1 Grammar production `Param ::= Identifier ':' TypeExpr \| TypeExpr`: bare identifier resolved in type namespace, unresolved reports E; unnamed signature requires lambda head to have parameter names (RFC-007:47 existing rule); RFC-010 form table and interface example (`(Surface)`) reconciled with batch | 8.3.2 + 6.8.1                    | Corpus survey zero migration (std/corpus bare identifiers all real types); unknown type identifier new E code    |

**Risk**: 8.4 dual parser diffing is **an unskippable equivalence checkpoint**. If it produces
inequality, it must be resolved before 8.5 stream switch, not concealed by C5′ after switch. 8.6's
~89 BP reference migrations if missed will **silently change associativity** (no error, only changes
parse result).

> **8.3–8.6 depend on `08`'s D0/D2**: if grammar file is placed outside `parser/`, or action code
> reverse-references `sema`, will be intercepted at `check-boundary.py`.

### P9 Anti-Rebound Gates (Unified Script List — Filled in by This Document for Original Gap G3)

Source: [01](01-routing.md) §Anti-Rebound Mechanism + [08](08-maintenance-mechanism.md)
§Machine-Checkable Rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts are integrated into
CI at their "introduction phase"; report-only ones are uniformly hard-closed in P9 (changed from
"report" to "fail") — existing violations (parallel representations, disambiguation aliases, etc.)
must wait for P6/P8 to clear; hardening too early will keep CI red long-term and have gates manually
bypassed; at P9 hardening, gates should be exactly all green. P2's 2.5, P4's 4.3.2, P7's 7c.1 all
reference this table, no separate list.

- [ ] **9.1–9.10 ten scripts integrated by introduction phase, uniformly hard-closed in P9**
      (details in table below)

| #    | Script                    | What it checks                                                                                                                  | Introduction Phase                                  | Acceptance                                                  |
| ---- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` bidirectional assertion                                                                      | P4                                                  | Intentionally missing one phase must go red                 |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third file read                                                              | P4 (**sole ownership, no longer duplicated in P9**) | Intentionally leaving one unconsumed field must go red      |
| 9.3  | `check-boundary.py`       | Prohibit `include!`; `pub(crate)` cross-layer leakage only decrease; prohibit L2→L3 (**sole ownership**, P0 of `08` defined it) | **P0**                                              | Intentionally adding `include!` must go red                 |
| 9.4  | `check-test-wiring.py`    | Has `tests/` directory but parent module has no `mod tests;` → fail                                                             | P1                                                  | Intentionally creating orphan directory must go red         |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message has no `snapshot-update` marker                                                            | P2                                                  | Same                                                        |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                            | P2                                                  | Same                                                        |
| 9.7  | `check-corpus-parity.py`  | Corpus diff non-empty (diagnostic code+span compared item by item, **no group relaxation**)                                     | P2                                                  | Same                                                        |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction outside `synth.rs`                                                                                     | P7                                                  | Same                                                        |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representation, disambiguation alias, synonym table)                                            | **P0** (report-only, hard-closed in P9)             | Must report 3 sets of operator enums on unchanged code      |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                             | **P0**                                              | Intentionally adding the 6th entry-style wiring must go red |

> **No line count ratchet script** (`check-file-size.py` + `baseline.toml` cancelled on 2026-10-03).
> Size issues are resolved by `08` prohibition 2 criterion A (responsibility separation, manual
> judgment).

### P10 Other Cleanup and Status Corrections

Source: [06](06-cleanup-inventory.md) §Implementation Notes

- [ ] **S2 Delete pure placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty design documents** (1 level-3 task)
- [ ] **S4 RFC status correction** (1 level-3 task)
- [ ] **S5 `pub` item visibility narrowing** (1 level-3 task)
- [ ] **S6 opcode dead path cleanup** (1 level-3 task)

| Level 2                              | Level 3                                                                                                                                                                                                                                                                                                                                                        | Acceptance                                                                                                                                                   |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S2 Delete pure placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead steps / old syntax detection / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: `lib.rs:568` / `:582` two `_ => todo!()` changed to meaningful degradation output**                                                                                                                 | `cargo test` passes; `clippy -D warnings` no new                                                                                                             |
| S3 Empty design documents            | After C1+C2 deletion, rewrite to point to `src/frontend/module/` (~2,670 lines); C3 rewrite boundary table; **sync cleanup `config.js:259-268`**                                                                                                                                                                                                               | No 404 on documentation site; `check-docs-truth.py` passes                                                                                                   |
| S4 RFC status correction             | C4 RFC-018 move back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 `TRACKING.md` add "Implementation Status" column (D36)                                                                                                                                                                                                                 | `check_tracking.py` exit code 0 (**don't manually modify `TRACKING.md`**)                                                                                    |
| S5 `pub` item visibility narrowing   | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be kept**)                                                                                                                                                                                                                                                                                  | Compiler can point out real dead code                                                                                                                        |
| S6 opcode dead path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete, opcode value reclaim already landed with version bump in 7e.1); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes `op` field); **F7+F8 opcode generation phase gate** (`tools/code-tables` expand opcode extractor ~30 lines + 5 places fact expression single-sourced + `size()` full opcode comparison test) | opcode fact 5-place expression converges to 1 place authoritative source; `size()` comment inconsistent with actual encoding (`bytecode.rs:2181-2182`) fixed |

## Implementation Notes: Dependencies and Parallelism

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P3.5 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 stop-bleeding channel (2026-10-05 decision)**: P3's prerequisite is narrowed to **2.3.2 +
> 2.4.1 + 2.4.3** (multi-file corpus layer + two red oracles). P2's remaining parts (2.1 IR
> verifier, 2.2 snapshots, 2.3.1/2.3.3 diff and performance baseline) can advance in parallel with
> P3. Correctness vulnerability stop-bleeding not blocked by oracle infrastructure construction; but
> **P4 must still wait for P2 to fully complete** — unified Driver needs all three layers of oracles
> in place.

| Constraint         | Reason                                                                                                                                                                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1            | Three prohibitions constrain every action in P1 (1.2.x is exactly adding `mod` declarations to `mod.rs`)                                                                                                                                    |
| P1 → P2            | Resurrecting tests will change test count and corpus baseline                                                                                                                                                                               |
| P2 (narrowed) → P3 | Vulnerability oracles must be **written red first** (2.4.1/2.4.3) + multi-file corpus layer (2.3.2) exists, red tests have place to run                                                                                                     |
| P3 → P4            | Fix bugs first then refactor; reverse order will let bugs be solidified into "established behavior" by the phase table                                                                                                                      |
| P3.5 → P4          | RFC-029g (#399) removal surface covers P4 4.2.2 / P5 / P6 / P7 / P8 respective rewrite targets (list in this document §P3.5) — reverse order equals redoing the same batch of removals on the new grammar, new checker layout, new IR (D56) |
| P4 → P5            | `include!` refactoring and checker split touch the same file                                                                                                                                                                                |
| P5 → P6            | Continuous changes to the same file must be separated, otherwise regression cannot be bisected                                                                                                                                              |
| P6 → P7 / P8       | If type representation is not first converged, new IR will grow into a third representation                                                                                                                                                 |
| P7/P8 → P9         | 9.5's snapshot baseline and 9.3's `pub(crate)` leakage count only take final form after P7/P8                                                                                                                                               |

### Parallelizable Groups

| Group                              | Members                                                                        | Basis                                                                                                                                                                                                                                |
| ---------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **A (sole one authorized by RFC)** | P7 ∥ P8                                                                        | File sets disjoint: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                        |
| B                                  | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                  | Each independently adds `mod`, revert file by file                                                                                                                                                                                   |
| C                                  | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                | Three layers of oracles independent of each other                                                                                                                                                                                    |
| D                                  | 4.4 (proof layer wrap-up) ∥ P6 entire line                                     | `02` explicitly states S5 "should walk independently of S1-S4"                                                                                                                                                                       |
| E                                  | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                          | All after 4.1 complete, mutually independent                                                                                                                                                                                         |
| F                                  | 9.3 ∥ 9.9 ∥ 9.10 (three scripts introduced by P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                                                                                                                                                           |
| G                                  | S3 ∥ S4 ∥ S5                                                                   | File sets disjoint (verified 2026-10-05: S3 touches `docs/src/dev/design/check/` + `config.js`; S4 touches `docs/src/rfc/` + `check_tracking.py` generator; S5 touches `src/frontend/core/typecheck/` two `pub`s; pairwise disjoint) |

**Not parallelizable (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5 same
batch file); 4.3.2 ∥ 9.2 (already unified to P4); **3.4 ∥ P3.5** (`checker.rs` / `environment.rs`
same file — external track's pub removal batch and silent channel fix must be serial).

**Internal document new constraint**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial; 7a→7b→7c→7d
strictly serial, and 7b's `Phi` arm must be before 7d.

## Key Decisions and Rationale

- **P0 as independent phase rather than merged into P9** — gates are "post-hoc check", rules are
  "ex-ante judgment". P9's scripts cannot prevent "should refactor but applied a patch instead",
  only D0–D4's review checklist can.
- **P9 as the sole authoritative list of CI scripts** — original gap G3's three coexisting lists led
  to inability to close. After this table merges, 10 scripts each own one phase, report-only and
  hard-close timing explicitly queryable.
- **P3.5 incorporates RFC-029g rather than leaving as "independent track, wait until after
  refactor"** — its removal surface overlaps with P4/P5/P6/P7/P8 rewrite surface (RFC-039 D56).
  Leaving for after refactor means redoing the same batch of removals on the new grammar, new
  checker layout and new IR; this is the same lesson as 7e incorporating `2315`/`2341`, S4
  incorporating RFC status correction.
- **P5's steps filled in by this document rather than merged into P4** — P4 is already the largest
  single-point risk, layering on checker split would exceed rollback granularity.
- **5.3.2 uses "zero caller changes" as hard acceptance** — split must be independently revertable.

## Known Limitations and Risks

- **1.3.x (fixing exposed defects) has no regression oracle** — only says "fixed defects should not
  be rolled back", lacks record requirement of "what was fixed". **Execution convention**: each
  1.3.x fix's commit message must reference the fixed test name and defect description, as post-hoc
  bisection basis.
- ~~**7b.1 granularity too coarse**~~ **Fixed (2026-10-05)**: split into 7b.1a / 7b.1b two exclusive
  commits per DoD 6.
- **8.2.1 granularity too coarse** — ~89 rename references, missed changes **silently change
  associativity**.
- ~~**D17 / D20 no corresponding level-3 tasks**~~ **Fixed (2026-10-05)**: D17 landed as 7e.1, D20
  landed as 4.5.1/4.5.2; D32/D34 opcode reclaim merged into 7e.1, deletion action left to P10 S6.
- ~~**`2315` / `2341` data loss defects originally "independent issue"**~~ **Fixed (2026-10-05)**:
  incorporated as 7e.2 / 7e.3. Exception table (throw/try) is language core semantics, must not be
  left as implementation legacy.
- ~~**E2 (`lib.rs:568/582` `todo!()`) no phase ownership**~~ **Fixed (2026-10-05)**: merged into P10
  S2.
- ~~**F7 (opcode gate extractor) ownership ambiguous**~~ **Fixed (2026-10-05)**: merged into P10
  S6's generation phase gate task.

## Conflict Register: All Adjudicated

> **Original "Conflict Register" C1–C8 and each document's open questions have all converged into
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).** This
> document has no pending items. Item-by-item correspondence:

| Original Number | Issue                                                      | Decision                                                                                                                                                        |
| --------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1              | `precedence_inline.rs` resurrect in P1, delete in P8       | **Resurrect** (P1's premise is resurrecting tests with real assertions), P8's 8.6 deletes this file along with Pratt                                            |
| C2              | "Grammar-driven" commitment vs tabulation                  | **Done completely**: adopted LALRPOP, see `05` "Syntax: Complete Grammar-Driven"                                                                                |
| C3              | C6 category definition and S1 classification conflict      | **S1 categorized into C6** (resurrection doesn't need equivalence oracle, only regression test); C6 definition expanded to "pure deletion or pure resurrection" |
| C4              | `test_release_plan_spans_consumed` whitelist               | **No whitelist**, difference set must be empty; if can't be empty means `ReleasePlan` contract has defect → change to `PlanId` (D20/D41)                        |
| C5              | `Aggregation` two function internal differences unverified | **P4's 4.2.7 has listed it as mandatory verification**, line-by-line confirmation during implementation                                                         |
| C6              | wasm playground fix needs P4's `ProgramKind`               | **wasm path vulnerability fix merged into 4.2.8**; P3 only handles three native entries                                                                         |
| C7              | `method_def_ordinals` ownership                            | **Belongs to 04** (P7 batch c), see D21                                                                                                                         |
| C8              | `upvalue_count` fix and `.42` version number               | **Fix + `VERSION` from 4 to 5** (format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`), see D17                                    |

## See Also

- [RFC-039 main text](../../rfc/accepted/039-compiler-architecture.md) — sole authority for phase
  sequence, DoD, global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 criterion category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0 detailed design and D0–D4 decision
  procedure
- [01-routing.md](01-routing.md) — target directory structure (post-completion form)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — each phase's design basis
