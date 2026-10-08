# Multi-Level Construction Task Table (WBS)

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md),
> expanding the RFC's phase sequence into **independently committable, independently verifiable**
> three-level tasks.
>
> The phase order and acceptance gates are uniquely authoritative in the RFC-039 body; this document
> only handles decomposition to executable granularity and registers undecided conflicts.

## Positioning and Scope

| Level       | Count       | Meaning                                                                                               |
| ----------- | ----------- | ----------------------------------------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Phases, in one-to-one correspondence with RFC-039's phase sequence                                    |
| **Level 2** | 46          | Task groups, from each document's "Implementation Key Points" + 2026-10-07 audit addendum (3.4 of P3) |
| **Level 3** | 130         | Independently committable actions                                                                     |

**This document covers**: task decomposition, dependencies, parallel groups, acceptance criteria
binding.

**This document does not cover**: the design rationale of tasks (see `02`–`08`), the argument for
phase order (see RFC-039).

## Current Status: Gaps Found During Decomposition

When consolidating the "Implementation Key Points" from 8 documents into this table, **5 hard gaps
in the documentation system itself** were found. All have been resolved:

| #   | Gap                                                                                                                                                                                                                                                                                                                                  | Resolution                                                                                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no construction steps**. RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents describes how to do this split; `01-routing.md`'s directory tree once mentioned `annotations.rs` as a line, which is the only mention in the whole batch                                                 | P5 steps are completed in this document (see below)                                                                                                                    |
| G2  | **`include!` refactoring three attributions, zero landed**. RFC lists it in P4; `02`'s cross-reference attributes it to itself; `06`'s S7 also lists it. But `02`'s change list (6 additions + 20 modifications) does **not include `semantic_tokens.rs`**                                                                           | Construction steps belong to this document §P5 5.1 (after P4 complete); RFC and 06's references have been redirected here                                              |
| G3  | **Three parallel CI script lists**. `RFC` lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists another 4 checks (all named differently); `04` also wants `check-synth-boundary.py`. After deduplication, **at least 9 new scripts, no unified list**; `check-obligations.py` is dual-attributed in P4 and P9 | Unified list in this document §P9 (10 scripts, each belonging to one phase), other documents uniformly reference this table                                            |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it in "1005 lines need resurrection", but the same document's B6 also says it must be "deleted simultaneously" with the `Precedence` enum; `05` advocates deletion                                                                                                   | **P1 resurrection, P8 deletion with Pratt** (decision registered C1) — first let CI regain coverage, the deletion action occurs in P8's 8.6 along with the dead ladder |
| G5  | **L2 goal statement conflict**. RFC and this directory index write "grammar-driven syntax", but `05` self-evaluates "**did not adopt true grammar-driven (LALRPOP), only achieved declarative table lookup**"                                                                                                                        | Converged to **LALRPOP complete grammar-driven** (05 §Syntax: Complete Grammar-Driven), G7/G8 acceptance gates executed per that commitment                            |

## Target Design: Phase Sequence

```
P0  Repository maintenance mechanism and code placement rules     08        ← New, establish rules first
P1  Revive orphan tests and fix defects           06 §S1
P2  Build equivalence criterion baseline             07 all
P3  Fix correctness vulnerabilities (minimal solution)        02 §S3-Vulnerability half
P4  Phase contract and unified Driver          02
P5  checker in-file split             02 + this document's supplement
P6  Type representation unification                 03
P7  Intermediate representation SSA-ification                04      } Parallel
P8  Frontend paradigm change                   05      }
P9  Anti-regression gates (including unified script list)     01 §Anti-regression + 08 §Machine-checkable rules
P10 Other cleanup and status correction             06 §S2/S3/S4/S5/S6
```

**Reason for the new P0**: The three prohibitions (must not invent / must not endlessly pad / should
refactor but patched) constrain every action of P1–P10. Rules must be established before work
begins, otherwise P1's test revival is already adding `mod` declarations to `tests/mod.rs` — which
is the cause of the G4 conflict.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [x] **0.1 Rules formalized** (3 level-3 tasks; 0.1.2/0.1.3 landed early on 2026-10-05; 0.1.1
      passed review on 2026-10-06: content verification + full Chinese localization of
      CONTRIBUTING.md, English version moved to docs/gh/CONTRIBUTING.en.md)
- [x] **0.2 Gate implementation** (3 level-3 tasks, completed 2026-10-06: check-concepts /
      check-fanout / check-boundary all connected to CI in report-only mode (concepts job), all
      negative acceptance probes passed (added entry wiring red / L2→L3 reverse use red / unchanged
      code reported 3 operator enums + 2 parallel type representations including ir::Type alias),
      pub(crate) baseline 137, unified to hard at P9)
- [ ] **0.3 Table gate promotion** (1 level-3 task)

| Level 2                  | Level 3                                                                                                                                                                                                                                                                                                                                        | Prerequisite | Acceptance                                                                                                                                                     |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Rules formalized     | 0.1.1 `CONTRIBUTING.md` writes D0–D4 + three prohibitions; responsibility categories reference `01-routing.md`'s directory responsibility table                                                                                                                                                                                                | —            | Manual review passed                                                                                                                                           |
|                          | 0.1.2 Implementer touchpoints landed (**completed 2026-10-05**): created `AGENTS.md` (root entry) + `compiler-architecture/HOWTO.md` (self-check before work + D3 patch judgment); PR template adds "Responsibility Attribution and Decision Procedure" required block (D44); `CONTRIBUTING.md` adds "Code Placement and Change Rules" section | —            | Landed; review verifies four touchpoint contents                                                                                                               |
|                          | 0.1.3 Rule body extraction (**completed 2026-10-05**): created `docs/src/dev/coding-rules.md` (precise rule description, long-term valid, not archived with RFC-039); 08 adds migration header note converted to diagnostic record; all four touchpoints redirect to coding-rules                                                              | —            | Landed                                                                                                                                                         |
| 0.2 Gate implementation  | 0.2.1 `check-concepts.py` (Prohibition 1 A/B/C/D)                                                                                                                                                                                                                                                                                              | 0.1.1        | **Must report 3 operator enums and 2 parallel type representations (including `ir::Type` alias) on unchanged code**; report-only at this stage, hardened at P9 |
|                          | 0.2.2 `check-fanout.py` (Prohibition 3 A/B/C)                                                                                                                                                                                                                                                                                                  | 0.1.1        | **Intentionally adding a 6th entry-style wiring must be red**; report-only at this stage, hardened at P9                                                       |
|                          | 0.2.3 `check-boundary.py` (Prohibition 2 criterion C)                                                                                                                                                                                                                                                                                          | 0.1.1        | **Intentionally adding an L2→L3 reverse `use` must be red**; `pub(crate)` leak count takes initial value                                                       |
| 0.3 Table gate promotion | 0.3.1 `tools/code-tables` extension covers opcode / type tables                                                                                                                                                                                                                                                                                | 0.1.1        | Depends on P6's T1; separate closure                                                                                                                           |

> **No size baseline task**. The 2026-10-03 decision canceled all line count / volume gates; size
> issues are resolved by responsibility separation (see `08` Prohibition 2 criterion A, manual
> judgment). **0.2.1 is the core acceptance for this stage**: if the gate cannot report known
> parallel representations, the criterion design is wrong, it's a fake gate.

### P1 Revive Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Key Points S1 + [05](05-frontend-paradigm.md)
§Test Reconstruction

- [x] **1.1 Revive `lexer/tests/`** (4 level-3 tasks, completed 2026-10-06: 55 lexer tests online;
      deleted 7 empty shells; `#[path]` bypass merged into proper wiring to avoid duplicate runs)
- [x] **1.2 Revive 4 places in active directories** (completed 2026-10-06: precedence_inline 6 +
      overload_inline 7 + json 3 + template 7; also deleted 5 lines of
      typecheck/tests/semantic_db.rs debris)
- [x] **1.3 Fix exposed defects** (completed 2026-10-06, **actual measurement differs from
      prediction**: overflow paths and \x/\u escapes are currently fine, 20 probe tests added for
      never-before coverage; the real exposed defect is "base literal followed by illegal
      alphanumeric adjacency silently split" — `0b102` compiles and runs with wrong value 2, `0o128`
      gets 10, `0x1FG`/`123abc` reports misleading E1001. Fix: `literals.rs` adds shared helper
      `reject_trailing_alnum` (Prohibition 3 compliant: one implementation, five call sites), 5
      red-then-green tests all green)

| Level 2                                   | Level 3                                                                                                                                                    | File:Line                  | Prerequisite | Acceptance                                                                     |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | ------------------------------------------------------------------------------ |
| 1.1 Revive `lexer/tests/`                 | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                        | After `lexer/mod.rs:106`   | —            | Test count increases                                                           |
|                                           | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                                | `lexer/tests/mod.rs:15-25` | 1.1.1        | **Without these, 159 lines still don't run**                                   |
|                                           | 1.1.3 Delete 7 empty shell files + their `mod` declarations                                                                                                | 7 files in `lexer/tests/`  | 1.1.1        | C6 no criterion                                                                |
|                                           | 1.1.4 Delete `pub use ...::*;` re-export block                                                                                                             | `tests/mod.rs:28-38`       | 1.1.3        | This block is exactly what was hiding the empty shells                         |
| 1.2 Revive 4 places in active directories | 1.2.1–1.2.4 respectively connect `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` | Each parent module         | —            | Test count increases; **1.2.1 conflicts with 8.0.1, see conflict register C1** |
| 1.3 Fix exposed defects                   | 1.3.1 Four base scanners overflow paths                                                                                                                    | `literals.rs`              | 1.1.x        | `cargo test` green                                                             |
|                                           | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                       | `literals.rs`              | 1.1.x        | Same as above                                                                  |

**Acceptance**: test count increases; **defects are expected to be exposed**, fix time must be
reserved. Revert file by file, but **fixed defects should not be rolled back**.

### P2 Build Equivalence Criterion Baseline

Source: [07](07-equivalence-oracle.md) all (this document has no "Implementation Key Points"
section, reverse-engineered from structure)

- [x] **2.1 IR static verifier** (3 level-3 tasks, all completed 2026-10-06: verify.rs dual mode + 7
      invariants + corpus green, actual measurement fixed ir_gen three defects)
- [x] **2.2 Normalized snapshots** (2 level-3 tasks, completed 2026-10-07: normalize.rs + 204
      snapshots committed + gate 9.5 added to CI)
- [x] **2.3 Corpus differential** (3 level-3 tasks, all completed 2026-10-07)
- [x] **2.4 Vulnerability-specific criteria** (3 level-3 tasks, all completed 2026-10-06:
      2.4.1/2.4.3 turn green with P3, 2.4.4 red state in place waiting for P4/D20)
- [ ] **2.5 Regression gate** (1 level-3 task)

| Level 2                             | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Prerequisite | Acceptance                                                                                 |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------ |
| 2.1 IR static verifier              | 2.1.1 `verify.rs` dual mode `verify_loose` / `verify_ssa` (**completed 2026-10-06**: `src/middle/core/verify.rs`, 07 §102 contract form `verify(ir, mode) -> Result<(), VerifyError>`, violations all collected, not first-error-then-stop; def/use extraction exhaustive match no wildcard arm, Phi variant introduction causes compilation failure to force patch check)                                                                                                                                                                                                                                             | —            | C4                                                                                         |
|                                     | 2.1.2 **`verify_loose` runs green on existing non-SSA IR** (**completed 2026-10-06**: `test_verify_loose_corpus_green` full corpus 204 files/536 functions/11990 instructions green, validation face non-empty assertion built-in; actual measurement fixed ir_gen three defects per D38, corpus differential baseline self-comparison zero differential proves behavioral equivalence)                                                                                                                                                                                                                                | 2.1.1        | **Hard threshold, entry condition for P7 batch a**——**unlocked**                           |
|                                     | 2.1.3 Implement 7 invariants (dominance/unique definition/Phi consistency/jump target/global out-of-bounds/type consistency/inner isolation) (**completed 2026-10-06**: determination face narrowed per actual measurement registration — dominance = named slot must-defined (temp slot Void pre-initialization is frame semantics, ir_gen intentionally relies on), type consistency = parameter slot signature + constant trusted source (ir_gen doesn't write slot type table, complete face belongs to P7 value table 04 §262), Phi waits for batch b variant introduction; see verify.rs file header and 06 §G6) | 2.1.1        | C4                                                                                         |
| 2.2 Normalized snapshots            | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / order predecessors) (**completed 2026-10-07**: `src/middle/core/normalize.rs`, 07 five rules implemented item by item; actual measurement corrected two places — DefId relativized by first appearance order (intern order unstable across processes, 141/143 actual measurement), Arg independent arg% prefix (sharing pool with Local loses variant distinction); instruction print exhaustive match)                                                                                                                                 | —            | C1/C3/C5                                                                                   |
|                                     | 2.2.2 Snapshot commit + manual review process (`src/middle/core/tests/snapshots/`) (**completed 2026-10-07**: 204 snapshots committed as relative paths mirroring corpus; update process `UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored` + git diff review + same PR commit, see tests/snapshot.rs file header; tampering test precisely reports red)                                                                                                                                                                                                                                                      | 2.2.1        | C1/C3/C5                                                                                   |
| 2.3 Corpus differential             | 2.3.1 Differential framework + 293 corpus baseline (**completed 2026-10-06**: probe `examples/corpus_probe.rs` + baseline `tests/baselines/corpus-parity.jsonl` (322 entries) + gate `check-corpus-parity.py` in CI (hard gate); normalization includes pointer form scrub, two full runs gate self-comparison zero differential)                                                                                                                                                                                                                                                                                      | —            | Diagnostic/exit code/stdout item by item                                                   |
|                                     | 2.3.2 **Multi-file corpus layer** (new `tests/yaoxiang-multifile/`, with `yaoxiang.toml` project fixture — decision D48) (**completed 2026-10-06**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | —            | **Unconditional must-do (D40), is the only executable behavioral criterion source for P4** |
|                                     | 2.3.3 C4 behavior differential must-cover list (10 semantic categories) + performance baseline (criterion smoke benchmark) (**completed 2026-10-07**: 10 semantic categories each item verified full coverage, 26 representative files 26/26 in corpus-parity.jsonl differential baseline; smoke benchmark `benches/pipeline.rs` first test — full corpus compile 17.6s / CLI cold start 169.7ms, interpreter throughput benches/lib.rs hotpath group already has coverage)                                                                                                                                            | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                                     |
| 2.4 Vulnerability-specific criteria | 2.4.1 `test_multifile_proof_obligation_not_dropped` (**completed 2026-10-06: red state in place**, actual measurement single file ["E4018"] vs compile_project [])                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 2.3.2        | **Must first be red**                                                                      |
|                                     | 2.4.3 `test_no_silent_pass_on_unproven` (**completed 2026-10-06: red state in place**, trigger source silently passes under compile_project in actual measurement)                                                                                                                                                                                                                                                                                                                                                                                                                                                     | —            | Block `checker.rs:5179/5318/5448`                                                          |
|                                     | 2.4.4 `test_release_plan_spans_consumed` (**completed 2026-10-06: red state in place**, actual measurement 168 files 373 keys not consumed — D20 contract defect evidence, fixing contract belongs to P4/D20)                                                                                                                                                                                                                                                                                                                                                                                                          | —            | **No whitelist, difference set must be empty (D41)**                                       |
| 2.5 Regression gate                 | 2.5.1–2.5.4 see P9 unified list                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 2.1–2.3      | See P9                                                                                     |

> **P2 landing (2026-10-06, 2.1)**: verify.rs dual mode + 7 invariants + full corpus green. **D38
> prediction fulfilled** — the process of running green actually measured and fixed three real
> defects in ir_gen (see 06 §G6 for details): method locals table truncated to parameter count
> (local_count underestimated, runtime relies on Frame::set_slot fallback), un-annotated parameters
> discarded by filter_map (three places, params metadata and frame layout misaligned), match result
> slot depends on frame implicit Void initialization (now explicitly pre-initialized). The realistic
> semantics of jump target is flattened instruction index (not 07 text's label keying), CFG built at
> instruction level — registered in verify.rs file header. **Commit correspondence**: ir_gen fix =
> `a9ea199b`; verify main body = `d5b18390` (⚠ parallel stream COMMIT_EDITMSG collision caused its
> message to be mistakenly labeled as "docs(rfc): RFC-027a…", content is actually verify.rs + six
> test files, buried in history intermediate cannot amend, hereby registered).
>
> **P2 narrowing part landing (2026-10-06)**: 2.3.2 + 2.4.1 + 2.4.3 all in place, **P3 stop-bleeding
> channel prerequisite unlocked**.
>
> - 2.3.2: `tests/yaoxiang-multifile/` 5 fixtures (behavior ×4 + compile-error ×1) + runner
>   `tests/yx_multifile_runner.rs`; judgment contract shares same set of `TestFileSpec`
>   (library-side single point) as single-file corpus, binary positioning/subprocess extraction
>   moved to `tests/common/` and yx_runner migrated simultaneously (Prohibition 3).
> - 2.4.1/2.4.3: `tests/integration/proof_obligations.rs`. Trigger source uses `SumUpTo(3, r)` form
>   nailed down by rfc027 tests (return position/call site/binding position) — 02's `Sorted(3)`
>   annotation example would be evaluated directly to Disproved in current code, not showing
>   Unproven silent, 07 pseudocode's Sorted(3) is only illustrative. Red state actual measurement
>   data in test file header.
> - Two red criteria hang `#[ignore]` permanently per 2.4.5 red skeleton precedent: default suite
>   all green, `-- --ignored` reproduces red state; **P3 fix lands when attribute removed turns
>   green**.
> - The intentional deviation from 07 (code, file, line) table in comparison normalization (remove
>   file/line, only compare error code set) is explained in 2.4.1 test comment.
>
> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **removed from P2**: both reference `Program` / `Obligations` types which only exist in P4, P2
> cannot compile. The former is P4's 4.1.4 (don't double-register), the latter's `#[ignore]` red
> skeleton merged into 4.3.1.
>
> **2.3.3 landing (2026-10-07)**: C4 must-cover list verified item by item (07 §143), 10 semantic
> categories corpus layer fully covered and all in corpus-parity.jsonl differential baseline (26
> representative files 26/26 hit, grep evidence) — ref/borrow/move 05-ownership entire directory
> (ref_shared, borrow_immutable/mutable/return, move_basic, ownership_deep etc.), closure captures
> closures + closure_arg_inference + spawn_capture, currying curry_value_fix + curry_tail_expr,
> spawn 04-concurrency ×13, iterator for spawn_for etc. 16 files, sum types sum_type__+
> pattern_or_guard + match_call_scrutinee, existential coercion point interface_dynamic_dispatch
> (Vec(Animal) existential heterogeneous packaging triggers existential_coercions) +
> interface_rebind_dispatch, `?`/Try propagation question_propagation + option_try + try_methods +
> user_try_type, method overload method_overload (+ method_overload_ambiguous_err in diagnostic
> differential face), refinement constraints Drop sequence refined__ ×5 + 05-ownership driven
> ReleasePlan (span key contract has 2.4.4 red criterion permanent). Performance baseline
> `benches/pipeline.rs` committed (07 §145 three smoke tests of two): `corpus_compile_frontend` full
> corpus compile first test 17.6s, `cli_cold_start_hello` process cold start 169.7ms; interpreter
> throughput covered by benches/lib.rs hotpath group (interp_fib_recursive_27 /
> interp_loop_steady_10m), no duplicate construction. Regression gate check-perf-regression.sh
> belongs to P9 unified list (2.5).

### P3 Fix Correctness Vulnerabilities (Minimal Solution)

Source: [02](02-stage-contract.md) §Implementation Key Points S3's vulnerability fix half +
**2026-10-07 silent drop point audit (3.4)**

- [x] **3.1 `proof_calls` consumer** (2 level-3 tasks, completed 2026-10-06)
- [x] **3.2 Second silent drop point** (1 level-3 task, completed 2026-10-06; **statement differs
      from actual measurement, see 3.2.1 row correction**)
- [x] **3.3 panic to diagnostic** (1 level-3 task, completed 2026-10-06)
- [ ] **3.4 Silent drop point addendum** (8 level-3 tasks, 2026-10-07 audit new; 3.4.1/3.4.2/3.4.3
      implemented, 3.4.4–3.4.8 advance with P4 prerequisite)

| Level 2                                               | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                 | File:Line                                                                                                  | Prerequisite | Acceptance                                                                                               |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------- |
| 3.1 `proof_calls` consumer                            | 3.1.1 `proof_calls` narrow visibility + unique getter (**2026-10-07 verification: actually `pub(super)` + `pub fn proof_calls()`, not "private"**)                                                                                                                                                                                                                                                                                      | `types.rs:36/84`                                                                                           | 2.4.1        | Vulnerability criterion turns green                                                                      |
|                                                       | 3.1.2 Consumer point completion                                                                                                                                                                                                                                                                                                                                                                                                         | `orchestrator.rs:99` / `:273` / `:450` / `:1374` (actually four entry points + single file total 5 places) | 3.1.1        | C2                                                                                                       |
| 3.2 Second silent drop point                          | 3.2.1 `Unproven` empty match arm produces diagnostic (**2026-10-07 verification: `ownership.rs` only constructs `Proved`/`Disproved`, this arm is unreachable in source code, is defensive code; criterion turn green is borne by 3.1, this item has no independent criterion**)                                                                                                                                                        | `checker.rs:1303-1314`                                                                                     | 2.4.3        | `test_no_silent_pass_on_unproven`                                                                        |
| 3.3 panic to diagnostic                               | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic                                                                                                                                                                                                                                                                                                                                                                                   | `predicate.rs:34-36`                                                                                       | —            | No longer panics when Z3 missing                                                                         |
| 3.4 Silent drop point addendum (2026-10-07 audit new) | 3.4.1 const generic constraint `Unproven` empty arm changes to produce diagnostic/record (**audit new: whole warehouse document zero registration, zero tasks**) (**implemented 2026-10-07**: wiring has registered W1063 — fact chain environment→ExpressionInferrer→StatementChecker→checker aggregate emission; bounds layer distinguishes TypeVar internal probe actual parameters to prevent false positives; red criterion first) | `environment.rs:422-426`                                                                                   | —            | New criterion red-then-green: const actual parameters must not be silent when not compile-time evaluable |
|                                                       | 3.4.2 Termination check solver missing leaves trace (**implemented 2026-10-07**: new code W1081 — `count_unjudged_obligations` pure function takes signal (decrement + well-founded two tables merged), checker aggregate emission; NotProved (solver present but cannot judge) not triggered, this code specific to "solver missing")                                                                                                  | `checker.rs:1266-1268`                                                                                     | —            | W-level diagnostic when Z3 missing; same degradation philosophy as 3.3.1                                 |
|                                                       | 3.4.3 02 §91 stage coverage table row 1 synchronization (**completed 2026-10-07**: table preserved as 9e02e4db diagnostic snapshot, review annotations cell by cell marked current status whereabouts; `#434` decision supplement registered as RFC-039 **D57**)                                                                                                                                                                        | `02:95-107`                                                                                                | —            | Manual review: cell by cell verified against code                                                        |
|                                                       | 3.4.4 `variant_ctor_calls` zero consumer decision (delete field or wire consumer; IR side currently relies on form detection `detect_variant_ctor_call`)                                                                                                                                                                                                                                                                                | `types.rs:56`, `ir_gen.rs:7374`                                                                            | 4.3.1        | `check-obligations.py` (9.2) no longer reports this field; no silent ambiguous form                      |
|                                                       | 3.4.5 `method_overload_ir_names` and `overload_resolutions` parallel representation choose one (Prohibition 1)                                                                                                                                                                                                                                                                                                                          | `types.rs:60`, `ir_gen.rs:369`                                                                             | 4.3.1        | Same fact single point expression; C3                                                                    |
|                                                       | 3.4.6 `compile_project` supplement `warnings` consumer (multi-file `run` never reports W1001/W1002/W1003)                                                                                                                                                                                                                                                                                                                               | `orchestrator.rs:99-159`                                                                                   | 4.2.1        | C2: multi-file and single-file diagnostic sets unified                                                   |
|                                                       | 3.4.7 LSP non-project-internal path access unified consumer (currently directly calls `check_module_collect_all`: doesn't run proof, doesn't read warnings, doesn't do dead code analysis)                                                                                                                                                                                                                                              | `lsp/handlers/diagnostics.rs:161-206`                                                                      | 4.2.6        | LSP and CLI have same diagnostic set for same file                                                       |
|                                                       | 3.4.8 Multi-file monomorphization arm (`instantiation_requests` only consumer in single file pipeline; cross-file generic matrix probe passes in actual measurement, is **potential** risk unproven defect)                                                                                                                                                                                                                             | `orchestrator.rs:174`, `pipeline.rs:330`                                                                   | 4.1.3        | C2: cross-file generic matrix equivalent to single file                                                  |

**Acceptance**: vulnerability criterion turns green, and **intentionally removing fix must turn red
again**. **Risk**: multi-file path adding E4018 is a breaking change.

**3.4 acceptance**: 3.4.1 / 3.4.2 must first have red criteria (prohibit fixing code first); 3.4.3
is manual review; 3.4.4–3.4.8 use C2/C3 and P9's 9.2 gate as standard. **Risk**: 3.4.1 new
diagnostic is a behavior change, must synchronize corpus differential baseline and CHANGELOG.

> **P3 landing (2026-10-06)**: 3.1.1 `proof_calls` narrow visibility + unique getter (then recorded
> as `pub(crate)`; **2026-10-07 verified actual landing is `pub(super)`** — code comment referenced
> `#434` decision has no registration on docs side, see 3.4.3; full private requires rewriting 11
> construction literals, stop-bleeding stage not taken; closed when P4 Obligations migration);
> execution mechanism extracted to `src/frontend/proof_execution.rs` (Prohibition 3 single point —
> cannot be placed in typecheck/, that would create L3→L4 reverse dependency); 3.1.2 per 02 evidence
> chain step 5 completes **four entry points** (WBS row number missed compile_project `:99`, must be
> completed and registered per 3.1.1 acceptance "criterion turn green"); 3.2.1 empty arm changes to
> record (proof_calls throws) + diagnostic (`into_result` established conversion path); 3.3.1 SOLVER
> slot Option-ized, Z3 missing degraded conservatively per `SMTResult::Unknown` (E2031 family
> diagnostic replaces process crash). Two vulnerability criteria remove `#[ignore]` turn normal
> green, supplement corpus fixture `proof-obligation-honored`; check-fanout inventory migrates
> accordingly (wiring point 19 = inventory). Actual measurement: criteria 2.4.1/2.4.3 turn green,
> full suite lib 2494 / integration 272 / corpus dual layer all green.
>
> **3.4 audit (2026-10-07, P3 landing re-review)**: for the category of "failure produces no signal"
> defects (definition see 02 §41-52), on unchanged code re-inventoried 8 more places, all registered
> as 3.4.x in above table. Methods and evidence:
>
> - **Consumer surface inventory** (16 obligation fields grep whole warehouse read points one by
>   one): `variant_ctor_calls` **zero consumer** — only read `checker.rs:1468` is "self-pour into
>   `TypeCheckResult`", IR side goes through form detection `ir_gen.rs:7374`; `rfc011b.rs:659/688`
>   only asserts production end, exactly the "only tests production end, naturally immune to
>   consumer end defect" form criticized in 02 §186 → 3.4.4. `method_overload_ir_names` result-level
>   field similarly zero external consumer, IR uses parallel `overload_resolutions` → 3.4.5.
> - **Stage coverage re-review**: 02 §91 table row 1 (four entry points "no proof_execution")
>   already fixed by 3.1.2, current state is 5 places share one implementation (`pipeline.rs:189`,
>   `orchestrator.rs:141/340/507/1448`); but rows 2/4/5 current state unchanged (multi-file `run`
>   still doesn't output W1003, doesn't run dead code family) → 3.4.6; LSP non-project-internal path
>   (02 §77 "manual lex→parse") similarly doesn't run proof → 3.4.7.
> - **Same-form empty arm scan** (`Unproven` whole warehouse consumers only four places): checker
>   three refinement branches (covered by 3.1), 3.2.1's ownership arm (**source unreachable**), and
>   `environment.rs:422` (**reachable, zero diagnostic, zero record, and whole warehouse docs zero
>   registration**) → 3.4.1, is the only "all-unknown" item in this audit.
> - **Solver acquisition surface**: `default_solver()` three consumer forms differ
>   (`predicate.rs:32` process singleton / `checker.rs:1266` per `check_module` injection /
>   `ownership.rs:621` new per back-edge judgment) — 3.3.1 only eliminated the panic; the other
>   two's **silent surface** belongs to 3.4.2, **singleton and cache hit** belong to P4's 4.4.2
>   (don't double-register).
> - **Not counted in 3.4 existing attributions**: 2.4.4 (`ReleasePlan` span key mismatch, 168
>   files/373 keys not consumed, Drop silently lost) is still P2 red state permanent, attribution
>   4.5.1/D20 unchanged; 4.4.1 (layer order declaration opposite of actual, `equivalence` layer not
>   in pipeline) / 4.4.3 (`unwrap_or_default()` empty ledger degradation) already in P4 register.
>
> **Execution convention**: 3.4.1 / 3.4.2 / 3.4.3 have no P4 prerequisite, can be landed in place at
> any time (recommend 3.4.1 first — the only "whole warehouse zero registration" live vulnerability,
> smallest change surface); 3.4.4–3.4.8 advance with P4 execution, their criteria depend on
> obligation ledger (4.3.1) or unified Driver (4.2.x). **3.4 does not establish new criterion
> system**, uniformly reuses existing C2/C3 and P9's 9.2 gate.
>
> **Stop-bleeding channel (2026-10-05 decision)**: P3 doesn't have to wait for P2 to complete.
> 3.1/3.2's only hard prerequisite is **2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red
> criteria)**; 2.1/2.2/2.3.1/2.3.3 (IR verifier, snapshots, single-file differential, performance
> baseline) can advance in parallel with P3. The stop-bleeding of correctness vulnerabilities
> (`Sorted(3)` silently passing) should not be blocked by snapshot infrastructure construction. 3.3
> (panic to diagnostic) has no prerequisite, can land at any time.

### P3.5 RFC-029g: Remove `pub` and Auto-Binding (External Track / #399 — Ordering Constraint D56)

Source: [RFC-029g](../rfc/accepted/029g-remove-pub-and-auto-bind.md) (accepted 2026-10-02,
implementation not started), [#399](https://github.com/ChenXu233/YaoXiang/issues/399). **This track
is not a design item of the refactoring itself, but a spec compliance deletion forcibly prerequisite
by ordering constraints**; and P10 S5 "`pub` item visibility downgrade" (Rust `pub(crate)`
narrowing) are two different things.

**Why must it overall precede P4**: 029g's deletion surface coincides with the rewrite surface of
four refactoring phases, reverse order = redoing the same batch of deletions and baseline updates on
new grammar / new checker layout / new SSA IR.

| 029g deletion surface                                              | File                                                                                                               | Which phase rewrites it                                                                                                                |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| `KwPub` and grammar branches                                       | `lexer/{tokens,state,mod}.rs`; `parser/parser_state.rs`, `statements/{declarations,imports,functions,bindings}.rs` | **P8** (8.2 lexer convergence / 8.3 build grammar / 8.6 delete Pratt / 8.7 split `parse_assign_after_target`)                          |
| AST `is_pub` ×2 (`Assign` / `TypeDefinition`)                      | `parser/ast.rs:247/270`                                                                                            | **P6 6.5 AST dead variant** + P8 + D1 directory rename (`parser/ast.rs` → top-level `src/ast/`)                                        |
| `auto_bind_to_type` and `collect_exports`'s pub branch             | `typecheck/checker.rs:2150/3802/3836`, `environment.rs:284`                                                        | **P5** (`include!`→real `mod` and checker split is **pure relocation**, pub branch will be moved into new file as-is)                  |
| Dead code exemption `exempt_pub` / `is_exported` and role dispatch | `typecheck/passes/dead_code.rs`, `module/orchestrator.rs:389-407` (inside `check_project` body)                    | **P4 4.2.2** (`check_project` slim down)                                                                                               |
| `semantic_tokens`'s `Public` modifier                              | `typecheck/checker/semantic_tokens.rs:450/498`                                                                     | **P4 4.2.6** + P5 + 4.6 (D55)                                                                                                          |
| `ir_gen`'s `is_pub` ignore binding                                 | `middle/core/ir_gen.rs:1378`                                                                                       | **P7** (`ir_gen.rs` SSA-ification)                                                                                                     |
| 31 `pub`s in `src/std/`                                            | `std/list.yx`(25) / `json.yx`(4) / `option.yx`(1) / `result.yx`(1)                                                 | No phase directly changes, but **P6 hard acceptance requires `git diff --stat tests/ src/std/` to be empty** — can only land before P6 |

**Side benefit**: 029g deletes `env.exports` dead table (production end fills table, whole warehouse
zero consumer) and `is_exported`/`is_visible` two accessors → P4's 4.3 obligation ledger has one
less "produced but no consumer" field.

- [ ] **3.5.1 Lexer and parser branches** (`KwPub` ×8)
- [ ] **3.5.2 AST `is_pub` ×2** (including production construction points and test fixture
      synchronization)
- [ ] **3.5.3 checker side** (`auto_bind_to_type`, `collect_exports`'s pub branch, `env.exports`
      dead table and two accessors)
- [ ] **3.5.4 Dead code exemption convergence** (`exempt_pub` / `is_exported` deletion;
      `orchestrator` role dispatch synchronization)
- [ ] **3.5.5 Periphery echo** (formatter's pub prefix, `semantic_tokens`'s `Public`)
- [ ] **3.5.6 std and diagnostic text** (31 pub prefixes; 4 W1xxx help "pub is external interface"
      statements change zh source, translation goes through bot)
- [ ] **3.5.7 Language reference convergence** (`modules.md` §3.2–3.3, `syntax.md` keyword table
      18→17, `warning-codes.md`, `language-overview.md`, `guide/modules.md`)

| Level 2                   | Level 3                                                                                                        | File:Line                                                                                                            | Acceptance                                                                                                                                              |
| ------------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 3.5.1 Lexer and parser    | 3.5.1.1 `KwPub` variant and `keyword_from_str` entry deletion                                                  | `lexer/tokens.rs:83`, `state.rs:28`, `mod.rs:62`                                                                     | Keyword table 18→17 (`Kw*` 16→15, including `tokens.rs:82` count comment)                                                                               |
|                           | 3.5.1.2 Declaration position pub detection and `pre_detected_pub` deletion                                     | `parser/statements/declarations.rs:672/675/731/747/880`, `functions.rs`, `bindings.rs`                               | `pub f = ...` reports parse error (**red criterion first**)                                                                                             |
|                           | 3.5.1.3 use curly brace entry pub skip and statement leading pub deletion                                      | `parser/statements/imports.rs:57`, `parser_state.rs:203`                                                             | `pub use m` reports parse error (**red criterion first**)                                                                                               |
| 3.5.2 AST                 | 3.5.2.1 `Assign`/`TypeDefinition`'s `is_pub` deletion + construction point synchronization                     | `parser/ast.rs:247/270` and each construction point                                                                  | No pub residue in compiler (`grep -rn is_pub src/` is empty)                                                                                            |
| 3.5.3 checker             | 3.5.3.1 `auto_bind_to_type` deletion (pub is the only call reason)                                             | `checker.rs:3802`, `environment.rs:284`, call site `checker.rs:2150`                                                 | Method form only has explicit composition (RFC-004), std and corpus zero dependency                                                                     |
|                           | 3.5.3.2 `collect_exports` remove pub branch + `env.exports` dead table and `is_exported`/`is_visible` deletion | `checker.rs:3836`, `environment.rs:599/612`                                                                          | Export surface unique authority = module registry `ModuleInfo.exports`; 9.2 gate one less field                                                         |
| 3.5.4 Dead code exemption | 3.5.4.1 `exempt_pub` / `is_exported` and exemption branch deletion                                             | `dead_code.rs:23/44/83/90/174/180/191/195/591/629`                                                                   | Lib/Script unreferenced top-level binding reports W1001 (**red criterion first**)                                                                       |
|                           | 3.5.4.2 `orchestrator` role dispatch synchronization                                                           | `orchestrator.rs:389-407`                                                                                            | Single file / multi-file diagnostic set unified (C2), W code not mixed into consumer                                                                    |
| 3.5.5 Periphery echo      | 3.5.5.1 formatter's pub prefix                                                                                 | `formatter/handlers/stmt.rs:81/117/146/151`                                                                          | format idempotent (existing criterion)                                                                                                                  |
|                           | 3.5.5.2 `semantic_tokens`'s `Public` modifier                                                                  | `typecheck/checker/semantic_tokens.rs:450/498`                                                                       | Semantic coloring no longer produces Public                                                                                                             |
| 3.5.6 std and text        | 3.5.6.1 Delete 31 pub prefixes in `src/std`                                                                    | `std/{list,json,option,result}.yx`                                                                                   | Corpus and std self-check all green; **must complete before P6**                                                                                        |
|                           | 3.5.6.2 Diagnostic text                                                                                        | 4 W1xxx help                                                                                                         | No longer claims "pub is external interface"; aligned with `dead_code.rs:574` Bin role behavior                                                         |
| 3.5.7 Documentation       | 3.5.7.1 Language reference and guide                                                                           | `reference/language-spec/{modules,syntax,language-overview}.md`, `warning-code/warning-codes.md`, `guide/modules.md` | Specification no longer has "pub still writable" statement opposite to implementation; `warning-codes.md:124` "pub never triggers W1001" also corrected |

**Acceptance (behavior change, red criterion first)**:

1. **Red-then-green**: Two behavior changes each first land red criterion then change code — ① `pub`
   from "silently accepted" to parse error; ② Lib/Script role unreferenced top-level binding newly
   reports W1001 (originally pub items absolutely exempt).
2. **Test count must not decrease**: when deleting `typecheck/passes/tests/dead_code.rs`'s
   `exempt_pub` cases and `is_pub` fixtures, must be replaced by equivalent new cases.
3. **Baseline same commit**: diagnostic set change (new parse error + new W1001) → corpus
   differential baseline (`tests/baselines/corpus-parity.jsonl`) and CHANGELOG same commit update;
   IR snapshot expected zero-diff, not zero must explain reason.
4. **Independent revert unit**: whole batch occupies one commit group, not mixed with other phases
   (DoD 6).
5. `cargo test` / `clippy -D warnings` / `python scripts/rfc/check_tracking.py` all green.

**Ordering**: overlaps with 3.4 files (`checker.rs`, `environment.rs`) → two serial, not parallel;
**overall precedes P4** (RFC-039 D56). P3.5 does not depend on 3.4, which to do first is determined
by implementer per on-hand context.

### P4 Phase Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Key Points S1–S5. **This phase and P3's fix must
be in two separate commits.**

- [ ] **4.1 Declarative phase table** (5 level-3 tasks)
- [ ] **4.2 Entry merge** (8 level-3 tasks)
- [ ] **4.3 Obligation ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm wrap-up** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ification (D20)** (2 level-3 tasks)
- [ ] **4.6 LSP semantic data pipeline unification (D55)** (1 level-3 task)

| Level 2                                         | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Prerequisite | Acceptance                                                                                                                         |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative phase table                     | 4.1.1 `stage.rs` (`Stage` variant + `Stage::ALL` + `StageScope`) (**completed 2026-10-07**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | P0           | `test_program_stage_coverage`                                                                                                      |
|                                                 | 4.1.2 `program.rs` / `unit.rs` (**completed 2026-10-07**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 4.1.1        | Same as above                                                                                                                      |
|                                                 | 4.1.3 `Pipeline::run` changes to Driver (`pipeline.rs:141-227`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | 4.1.2        | **Single file diagnostic set and exit code byte-for-byte identical**                                                               |
|                                                 | 4.1.4 Stage coverage assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 4.1.3        | C2                                                                                                                                 |
|                                                 | 4.1.5 Full corpus zero-diff                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 4.1.4        | Diagnostic/exit code/stdout all zero-diff (293 corpus)                                                                             |
| 4.2 Entry merge                                 | 4.2.1 `compile_project` slimmed to Program constructor (`orchestrator.rs:99-237`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | 4.1.5        | Four entry diagnostic sets identical after unification                                                                             |
|                                                 | 4.2.2 `check_project` (`:273-399`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | 4.2.1        | Same as above                                                                                                                      |
|                                                 | 4.2.3 `check_source_in_project` (`:450`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | 4.2.2        | Same as above                                                                                                                      |
|                                                 | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 4.2.3        | Same as above                                                                                                                      |
|                                                 | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 4.2.1        | `check` consistent inside and outside project                                                                                      |
|                                                 | 4.2.6 Delete LSP manual phase sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | 4.2.1        | LSP and CLI diagnostic set identical                                                                                               |
|                                                 | 4.2.7 `Aggregation` parameter-driven two-choice (`orchestrator.rs:486-494`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 4.2.1        | **Verify two function internal differences line by line during implementation** (conflict register C5 decided as must-verify item) |
|                                                 | 4.2.8 wasm changes to `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | 4.2.1        | See C6                                                                                                                             |
| 4.3 Obligation ledger                           | 4.3.1 `obligations.rs` + `assert_drained()` (newly created; includes `#[ignore]` red skeleton first)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 4.1.3        | `test_obligations_drained` turns green                                                                                             |
|                                                 | 4.3.2 Obligation diagnostic W→E upgrade                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 4.3.1        | Manual review each new E                                                                                                           |
| 4.4 Proof layer and wasm wrap-up                | 4.4.1 `layers/README.md` layer order change to actual order                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | —            | **Walks independently of 4.1–4.3**                                                                                                 |
|                                                 | 4.4.2 SMT solver acquisition surface unified as process-level shared singleton (**per D58**: real fix location `backend.rs` — `LazyLock<Mutex<Option<Box<dyn Solver>>>>` + `with_shared_solver` closure port; `predicate.rs` delete private SOLVER, `checker.rs` termination injection change to in-closure judgment (`TerminationChecker` lifetime parameterization), `ownership.rs` back-edge judgment change to shared port; `default_solver()` only retained for testing. **Hard prerequisite**: Unknown doesn't enter cache (otherwise timeout result solidifies across process-level cache across compilation/tests). 89576fafd only delivered counter observation tool, this task body untouched) | —            | Production path `default_solver()` call point zero + cache hit rate observable across three consumer points                        |
|                                                 | 4.4.3 Degradation path add warning (`checker.rs:1283-1293`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | —            | —                                                                                                                                  |
| 4.5 Cross-layer contract PlanId-ification (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (`layers/ownership.rs:31` production side allocation + `ir_gen.rs:1942` consumer side matching)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | 4.3.1        | `test_release_plan_spans_consumed` difference set empty (D41)                                                                      |
|                                                 | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | 4.5.1        | C2; span mismatch type silent failure zero                                                                                         |
| 4.6 LSP semantic data pipeline unification      | 4.6.1 Project-internal file semantic data (SemanticDB) goes through orchestrator same pipeline, same source as diagnostic; cross-file reference carries `resolves_to`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | 4.2.1        | Project-internal cross-file jump hits definition; semantic data and diagnostic from same compilation                               |

### P5 checker In-File Split — This Document Supplements (Original Gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (`collect_used_in_type`'s
`pub(crate)` path must not change)

- [ ] **5.1 `include!` → Real `mod`** (2 level-3 tasks)
- [ ] **5.2 Split out `refinement`** (2 level-3 tasks)
- [ ] **5.3 Split out `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                              | Prerequisite | Acceptance                                                  |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------------- |
| 5.1 `include!` → Real `mod` | 5.1.1 `checker.rs:5618`'s `include!` change to real `mod` declaration                                                                                                                                                                                | P4 complete  | C1 zero-diff; `semantic_tokens.rs` gets module identity     |
|                             | 5.1.2 Add `use super::*` or item-by-item introduction (`include!` era implicit inherited scope)                                                                                                                                                      | 5.1.1        | `cargo build` green                                         |
| 5.2 Split out `refinement`  | 5.2.1 Migrate out refinement block (currently `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                   | 5.1.1        | C1 zero-diff                                                |
|                             | 5.2.2 `collect_refined_binding_checks` signature refactor: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                               | 5.2.1        | C1; call site only `1235`                                   |
| 5.3 Split out `annotations` | 5.3.1 Migrate out annotation check (currently `3441-3661`, including `is_predicate_head`)                                                                                                                                                            | 5.2.2        | C1 zero-diff                                                |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) keep `pub(crate)` and under `checker` module path** → add `pub(crate) use annotations::*;` at top of `checker.rs`, making `inference/statements.rs:16`'s import **unchanged by one character** | 5.3.1        | **Zero caller change** is the hard acceptance for this step |

**Hard constraint**: `collect_used_in_type` is used cross-module by `inference/statements.rs:16`.
The approach is `pub(crate) use` re-export, making the split a **pure relocation**.

**Only split confirmed large blocks.** Don't split modules that are only 200–400 lines — increases
navigation cost, reduces cohesion, zero decoupling benefit. Other submodules (signatures / type_defs
/ imports) wait for file to grow back to 1,500 lines before splitting.

### P6 Type Representation Unification

Source: [03](03-type-unification.md) §Implementation Key Points Phases 0–6, criterion C3. **Phases
0→6 strictly serial.**

- [ ] **6.0 Gate first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead variant deletion** (2 level-3 tasks)
- [ ] **6.4 parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variant** (2 level-3 tasks)
- [ ] **6.6 Gate hardened** (1 level-3 task)
- [ ] **6.7 Directory rename (D1, pure relocation batch)** (1 level-3 task)
- [ ] **6.8 Annotation check mode and Fn parameter representation (D54, 2026-10-05 supplement)** (2
      level-3 tasks)

| Level 2                                                         | Level 3                                                                                                                                                                                                                                                                                                                                                                                  | File:Line                                                                 | Acceptance                                                                                                                                                                                                  |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.0 Gate first                                                  | 6.0.1 New `tools/type-tables`, implement T1/T2/T3, **T1 first report only doesn't fail**                                                                                                                                                                                                                                                                                                 | New crate                                                                 | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge rebuild point list**                                                                                                           |
|                                                                 | 6.0.2 Take full baseline on unchanged code                                                                                                                                                                                                                                                                                                                                               | —                                                                         | No code change                                                                                                                                                                                              |
| 6.1 Name normalization                                          | 6.1.1 Count corpus non-canonical type name occurrences                                                                                                                                                                                                                                                                                                                                   | `tests/yaoxiang/`                                                         | Decide whether synonyms can be directly deleted                                                                                                                                                             |
|                                                                 | 6.1.2 `from_builtin_name` chop synonyms → lexer alias table                                                                                                                                                                                                                                                                                                                              | `mono.rs:618-643`                                                         | C3                                                                                                                                                                                                          |
|                                                                 | 6.1.3 Verify `classify_generic_params` consumes which parameter list                                                                                                                                                                                                                                                                                                                     | `types.rs:162-165`                                                        | **Must verify line by line, cannot assume equivalent**                                                                                                                                                      |
| 6.2 Bytecode type unification                                   | 6.2.1 Three places change `MonoType`                                                                                                                                                                                                                                                                                                                                                     | `bytecode.rs:812/814/854`, `image.rs:44`                                  | C3 + `dump_bytecode` directed comparison                                                                                                                                                                    |
|                                                                 | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                                                                                                                                                                                                 | `bytecode.rs:2352-2390`                                                   | Same as above                                                                                                                                                                                               |
|                                                                 | 6.2.3 `type_table` element type change to `MonoType`                                                                                                                                                                                                                                                                                                                                     | `image.rs:44`                                                             | Same as above                                                                                                                                                                                               |
| 6.3 Dead variant deletion                                       | 6.3.1 Delete 11 variants + clean match arm + fallback                                                                                                                                                                                                                                                                                                                                    | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                               | C3 + `cargo build` green                                                                                                                                                                                    |
|                                                                 | 6.3.2 **Must be same batch as 6.3.1** rewrite reverse bridge                                                                                                                                                                                                                                                                                                                             | `passes/mono/function.rs:507-529` (`mono_to_ast_type` rebuilds 7 of them) | Generic substitution directed comparison, otherwise **silently change behavior**                                                                                                                            |
| 6.4 parser data flow                                            | 6.4.1 New `probe.rs`                                                                                                                                                                                                                                                                                                                                                                     | New creation                                                              | C3                                                                                                                                                                                                          |
|                                                                 | 6.4.2 Change `parser_state.rs:46-58`                                                                                                                                                                                                                                                                                                                                                     | —                                                                         | Same as above                                                                                                                                                                                               |
|                                                                 | 6.4.3 Delete `operator_interfaces::spec()` call (**2 places**: `declarations.rs:509`, `ast.rs:930`)                                                                                                                                                                                                                                                                                      | —                                                                         | Same as above                                                                                                                                                                                               |
|                                                                 | 6.4.4 Merge two `name_used_as_type*`                                                                                                                                                                                                                                                                                                                                                     | `ast.rs:847` + `declarations.rs:42`                                       | Same as above                                                                                                                                                                                               |
|                                                                 | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) via `NameKind::Builtin` wiring                                                                                                                                                                                                                                                                                                                  | —                                                                         | Same as above                                                                                                                                                                                               |
| 6.5 AST dead variant                                            | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                                                                                                                                                                                                               | `ast.rs:37-43` + 4 consumer + 2 exhaustive arm                            | `tests/integration/` 18 modules green (**split into 2 commits**)                                                                                                                                            |
|                                                                 | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                                                                                                                                                                                                          | `ast.rs:241-249`                                                          | Mark **semantic risk**                                                                                                                                                                                      |
| 6.6 Gate hardened                                               | 6.6.1 T1 change from warning to `panic!`                                                                                                                                                                                                                                                                                                                                                 | `tools/type-tables`                                                       | **Intentionally introducing zero-construction variant must be red**                                                                                                                                         |
| 6.7 Directory rename (D1, pure relocation batch)                | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; `parser/ast.rs` relocate to top-level `ast/`; bytecode domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` move to `middle/bytecode/`                                                                                                                                                                                  | Whole warehouse                                                           | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline doesn't regress                                                                                                                   |
| 6.8 Annotation check mode and Fn parameter representation (D54) | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>` (name and type live together), same batch closure as 6.5.2 delete `Assign.signature_params` — 6.5.2 only deletes old representation, this item provides alternative representation                                                                                                                                                     | `ast.rs:451-454`, consumer surface about 20 files (mechanical change)     | C3 (pure representation refactoring batch, comparison not enabled); split into independent commit                                                                                                           |
|                                                                 | 6.8.2 Declaration-driven check mode: parameter types positionally drive lambda header (independent of name), exit (tail expression and return) unified comparison annotation return type, non-lambda value infer+unify (same structure as `m: Int = "hello"`); red criterion first — u1–u4/v1–v5 probes into corpus (red-then-green); delete `declarations.rs:337-376` parser name merge | `statements.rs:1170-1914`                                                 | Comparison enabled batch is **behavior fix**: new diagnostic code, baseline update, C3 doesn't apply; v1–v5 five syntax forms same judgment (eliminate "same semantics different syntax one red one green") |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA-ification

Source: [04](04-ssa.md) §Implementation Key Points batches a–d, criterion C4, **strictly serial
within batch**.

- [ ] **7a Cut off multiple definitions** (3 level-3 tasks)
- [ ] **7b SSA form switch** (6 level-3 tasks; 7b.1 split into 1a/1b per DoD 6)
- [ ] **7c Implicit contract explicitation** (3 level-3 tasks)
- [ ] **7d Last** (2 level-3 tasks)
- [ ] **7e `.42` data loss fix** (3 level-3 tasks; 2315/2341 absorbed from "independent issue",
      2026-10-05)

| Level 2                            | Level 3                                                                                                                                                | Prerequisite                        | Acceptance                                         |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------- | -------------------------------------------------- |
| 7a Cut off multiple definitions    | 7a.1 Delete 3 statement-level recovery                                                                                                                 | **2.1.2 `verify_loose` runs green** | C4                                                 |
|                                    | 7a.2 6 places save/restore change to RAII guard                                                                                                        | 7a.1                                | C1 snapshot zero-diff                              |
|                                    | 7a.3 Caliber unification (`1798` vs `2001`)                                                                                                            | 7a.2                                | C1                                                 |
| 7b SSA form switch                 | 7b.1a `ir.rs` structure change #1-2 (`Operand` variant convergence + `Instruction::Phi` introduction)                                                  | 7a.3 + **P6 complete**              | C4; exclusive commit                               |
|                                    | 7b.1b `ir.rs` structure change #3-5 (`FunctionIR` / `BasicBlock` / type annotation field)                                                              | 7b.1a                               | C4; exclusive commit                               |
|                                    | 7b.2 `next_temp_reg` change to `Operand::Value`                                                                                                        | 7b.1                                | Same as above                                      |
|                                    | 7b.3 `translator.rs` add `Phi` arm                                                                                                                     | 7b.1                                | C4 + `.42` round-trip test                         |
|                                    | 7b.4 Linear scan allocator (within u8/255 slot model)                                                                                                  | 7b.3                                | Same as above (+500~1000 lines)                    |
| 7c Implicit contract explicitation | 7c.1 `synth.rs` boundary + boundary check script                                                                                                       | 7b.4                                | C4 + new script into CI                            |
|                                    | 7c.2 span consumption count                                                                                                                            | 7c.1                                | Consumption count **zero trigger**                 |
|                                    | 7c.3 `method_def_ordinals` read-only                                                                                                                   | 7c.1                                | Attribution questionable (C7)                      |
| 7d Last                            | 7d.1 `generate_call_expr_ir` split into `CallArgs` + 6 `emit_*`                                                                                        | 7c.x                                | C1 snapshot zero-diff                              |
|                                    | 7d.2 "补 0" fallback change to return diagnostic                                                                                                       | 7d.1                                | C4                                                 |
| 7e `.42` data loss fix             | 7e.1 `upvalue_count: 0` correction (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + already allocated unused opcode reclaim (D32/D34 with version upgrade) | 7d.x                                | C4 + `.42` round-trip test; exclusive commit       |
|                                    | 7e.2 `exception_handlers` persist (`bytecode.rs:2315` — throw/try are language core semantics, **must not be left for implementation**)                | 7e.1                                | `.42` direct run exception case differential       |
|                                    | 7e.3 `globals` persist (`bytecode.rs:2341`)                                                                                                            | 7e.1                                | `.42` direct run global variable case differential |

**Expectation management**: lines **net increase 900–1600 lines**. If "code becomes shorter" is used
as the success criterion at project initiation, this stage will be judged as failure.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Key Points Phases 0–4, criterion C5.

- [ ] **8.0 Dead ladder cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexer convergence** (2 level-3 tasks)
- [ ] **8.3 Build LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual parser differential** (1 level-3 task)
- [ ] **8.5 Cutover** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 `parse_assign_after_target` split** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)
- [ ] **8.9 Parameter position form unification (D53, 2026-10-05 supplement)** (1 level-3 task)

| Level 2                                       | Level 3                                                                                                                                                                                                                                                                                                                   | Prerequisite                     | Acceptance                                                                                                        |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead ladder cleanup (C6)                  | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                                                                                                                                                                                                                                           | **Conflicts with 1.2.1, see C1** | C6 no criterion                                                                                                   |
|                                               | 8.0.2 Bare magic numbers name-ized (`(6,7)` / `(11,1)` / `12`)                                                                                                                                                                                                                                                            | —                                | Same as above                                                                                                     |
| 8.2 Lexer convergence                         | 8.2.1 Four base scanners merge + three escape merges + multi-line string merge                                                                                                                                                                                                                                            | —                                | C5; `literals.rs` reduce about 500 lines                                                                          |
|                                               | 8.2.2 f-string nested compilation elimination                                                                                                                                                                                                                                                                             | 8.2.1                            | **span must change**, baseline must be built first                                                                |
| 8.3 Build LALRPOP grammar                     | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                                                                                                                                                                                                                                             | 8.2.x                            | `cargo build` passes; can produce AST for 293 corpus (**not comparison**)                                         |
|                                               | 8.3.2 Action code `grammar/actions.rs` (22 `Expr` variants each one function)                                                                                                                                                                                                                                             | 8.3.1                            | Same as above                                                                                                     |
|                                               | 8.3.3 Error production (preserve `Expr::Error` / `StmtKind::Error` placeholder semantics)                                                                                                                                                                                                                                 | 8.3.2                            | Same as above                                                                                                     |
| 8.4 **Dual parser differential**              | 8.4.1 Two parsers each run 293 corpus + `src/std/tests`, after AST normalization **bit-by-bit comparison**                                                                                                                                                                                                                | 8.3.3                            | **Legal program AST fully equivalent — this is the equivalence proof for the entire grammar migration**           |
| 8.5 Cutover                                   | 8.5.1 `parse()` change to call LALRPOP, Pratt retained as `parse_legacy()`                                                                                                                                                                                                                                                | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span each item identical (**no relaxation**, no C5′ exists)     |
| 8.6 Delete Pratt                              | 8.6.1 Delete `nud.rs`(1326) + `led.rs`(451) + two BP ladders + about 89 references                                                                                                                                                                                                                                        | 8.5.1                            | Corpus all green; `git grep BP_` zero hit                                                                         |
|                                               | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally invalid)                                                                                                                                                                                                                                                       | 8.6.1                            | Same as above                                                                                                     |
| 8.7 `parse_assign_after_target` split         | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                                                                                                                                                                                                                                                    | 8.6.x                            | This function only does dispatch                                                                                  |
| 8.8 Test completion                           | 8.8.1 New associativity case (**red-then-green**)                                                                                                                                                                                                                                                                         | 8.2.x                            | C5                                                                                                                |
|                                               | 8.8.2 Supplement four literal error paths                                                                                                                                                                                                                                                                                 | —                                | Cover untested paths                                                                                              |
|                                               | 8.8.3 Supplement `pratt/tests/mod.rs` wiring self-check assertion                                                                                                                                                                                                                                                         | —                                | Prevent orphan recurrence (**`pratt/tests/` doesn't exist after 8.6, change to `parser/tests/`**)                 |
| 8.9 Parameter position form unification (D53) | 8.9.1 Grammar production `Param ::= Identifier ':' TypeExpr \| TypeExpr`: bare identifier resolves in type namespace, unresolved reports E; unnamed signature requires lambda header to bring own parameter name (RFC-007:47 existing rule); RFC-010 form table and interface example (`(Surface)`) reconciled with batch | 8.3.2 + 6.8.1                    | Corpus census zero migration (std/corpus bare identifiers all are real types); unknown type identifier new E code |

**Risk**: 8.4 dual parser differential is **an unavoidable equivalence gate**. If it produces
non-equivalence, must be resolved before 8.5 cutover, not covered up by C5′ after cutover. 8.6's
about 89 BP reference migration, if missed, will **silently change associativity** (no error, only
change parse result).

> **8.3–8.6 depend on `08`'s D0/D2**: if grammar file is placed outside `parser/`, or action code
> reverse-references `sema`, will be blocked at `check-boundary.py`.

### P9 Anti-Regression Gate (Unified Script List — This Document Supplements Original Gap G3)

Source: [01](01-routing.md) §Anti-regression mechanism + [08](08-maintenance-mechanism.md)
§Machine-checkable rules

**This is the only authoritative list of all CI scripts** (10 scripts). Scripts are connected to CI
at the "introduction stage"; report-only ones are uniformly hardened at P9 (change from "report" to
"fail") — existing violations (parallel representations, disambiguation aliases etc.) can only be
cleared after P6/P8, hardening early will make CI long-term red, gates manually bypassed, when P9
hardens gates should be exactly all green. P2's 2.5, P4's 4.3.2, P7's 7c.1 all reference this table,
no separate list.

- [ ] **9.1–9.10 ten scripts connected per introduction stage, uniformly hardened at P9** (details
      in table below)

| #    | Script                    | What it checks                                                                                                                       | Introduction stage                                    | Acceptance                                              |
| ---- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------- | ------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` bidirectional assertion                                                                           | P4                                                    | Intentionally missing a stage must be red               |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third file read                                                                   | P4 (**only attribution, no longer duplicates in P9**) | Intentionally leaving a no-consumer field must be red   |
| 9.3  | `check-boundary.py`       | Forbid `include!`; `pub(crate)` cross-layer leak only allowed to decrease; forbid L2→L3 (**only attribution**, `08`'s P0 defines it) | **P0**                                                | Intentionally adding `include!` must be red             |
| 9.4  | `check-test-wiring.py`    | Has `tests/` directory but parent module no `mod tests;` → fail                                                                      | P1                                                    | Intentionally creating orphan directory must be red     |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message no `snapshot-update` mark                                                                       | P2                                                    | Same as above                                           |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                                 | P2                                                    | Same as above                                           |
| 9.7  | `check-corpus-parity.py`  | Corpus differential non-empty (diagnostic code+span bit-by-bit comparison, **no group relaxation**)                                  | P2                                                    | Same as above                                           |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction appears outside `synth.rs`                                                                                  | P7                                                    | Same as above                                           |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representation, disambiguation alias, synonym table)                                                 | **P0** (report-only, hardened at P9)                  | Must report 3 operator enums on unchanged code          |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                                  | **P0**                                                | Intentionally adding 6th entry-style wiring must be red |

> **No line count ratchet script** (`check-file-size.py` + `baseline.toml` canceled on 2026-10-03).
> Size issue resolved by `08` Prohibition 2 criterion A (responsibility separation, manual
> judgment).

### P10 Other Cleanup and Status Correction

Source: [06](06-cleanup-inventory.md) §Implementation Key Points

- [ ] **S2 Delete pure placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty design document** (1 level-3 task)
- [ ] **S4 RFC status correction** (1 level-3 task)
- [ ] **S5 `pub` item visibility downgrade** (1 level-3 task)
- [ ] **S6 opcode dead path cleanup** (1 level-3 task)

| Level 2                              | Level 3                                                                                                                                                                                                                                                                                                                                                     | Acceptance                                                                                                                                        |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| S2 Delete pure placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladder / old syntax detection / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: `lib.rs:568` / `:582` two places `_ => todo!()` change to meaningful degradation output**                                                                                                       | `cargo test` passes; `clippy -D warnings` no new                                                                                                  |
| S3 Empty design document             | C1+C2 deleted rewrite to point to `src/frontend/module/` (about 2,670 lines); C3 rewrite boundary table; **synchronously clean `config.js:259-268`**                                                                                                                                                                                                        | Documentation site no 404; `check-docs-truth.py` passes                                                                                           |
| S4 RFC status correction             | C4 RFC-018 move back to `draft/` (D46); C5 RFC-028 supplement `impl_status: 'not-started'`; F3 `TRACKING.md` add "implementation status" column (D36)                                                                                                                                                                                                       | `check_tracking.py` exit code 0 (**don't manually edit TRACKING.md**)                                                                             |
| S5 `pub` item visibility downgrade   | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be preserved**)                                                                                                                                                                                                                                                                          | Compiler can point out real dead code                                                                                                             |
| S6 opcode dead path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete, opcode value reclaim landed in 7e.1 with version upgrade); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes `op` field); **F7+F8 opcode generation-time gate** (`tools/code-tables` extend opcode extractor about 30 lines + 5 place fact expression single-source + `size()` full opcode comparison test) | opcode fact 5-place expression converge to 1 place authority source; `size()` comment doesn't match actual encoding (`bytecode.rs:2181-2182`) fix |

## Implementation Key Points: Dependencies and Parallelism

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P3.5 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 stop-bleeding channel (2026-10-05 decision)**: P3's prerequisite narrowed to **2.3.2 +
> 2.4.1 + 2.4.3** (multi-file corpus layer + two red criteria). P2's other parts (2.1 IR verifier,
> 2.2 snapshots, 2.3.1/2.3.3 differential and performance baseline) can advance in parallel with P3.
> The stop-bleeding of correctness vulnerabilities is not blocked by criterion infrastructure
> construction; but **P4 still must wait for P2 to fully complete** — unified Driver needs all
> three-layer criteria in place.

| Constraint         | Reason                                                                                                                                                                                                                                     |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P0 → P1            | Three prohibitions constrain every action of P1 (1.2.x is adding `mod` declarations to `mod.rs`)                                                                                                                                           |
| P1 → P2            | Reviving tests will change test count and corpus baseline                                                                                                                                                                                  |
| P2 (narrowed) → P3 | Vulnerability criteria must **first be written red** (2.4.1/2.4.3) + multi-file corpus layer (2.3.2) exists, red tests have place to run                                                                                                   |
| P3 → P4            | Fix bugs first then refactor; reverse order will let bugs be solidified as "established behavior" by phase table                                                                                                                           |
| P3.5 → P4          | RFC-029g (#399)'s deletion surface covers P4 4.2.2 / P5 / P6 / P7 / P8's respective rewrite objects (list see this document §P3.5) — reverse order equals redoing same batch of deletions on new grammar, new checker layout, new IR (D56) |
| P4 → P5            | `include!` refactoring and checker split move same file                                                                                                                                                                                    |
| P5 → P6            | Consecutive changes to same file must be separated, otherwise regression cannot be bisected                                                                                                                                                |
| P6 → P7 / P8       | Type representation not converged first, new IR will grow into third representation                                                                                                                                                        |
| P7/P8 → P9         | 9.5's snapshot baseline and 9.3's `pub(crate)` leak count take final form after P7/P8                                                                                                                                                      |

### Parallel Groups

| Group                          | Members                                                                        | Basis                                                                                                                                                                                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A (only authorized by RFC)** | P7 ∥ P8                                                                        | Disjoint file sets: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                                |
| B                              | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                  | Each independently adds `mod`, revert file by file                                                                                                                                                                                           |
| C                              | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                | Three-layer criteria independent of each other                                                                                                                                                                                               |
| D                              | 4.4 (proof layer wrap-up) ∥ P6 entire line                                     | `02` explicitly says S5 "should walk independently of S1-S4"                                                                                                                                                                                 |
| E                              | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                          | All after 4.1 completes, no dependencies between each other                                                                                                                                                                                  |
| F                              | 9.3 ∥ 9.9 ∥ 9.10 (three scripts introduced at P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                                                                                                                                                                   |
| G                              | S3 ∥ S4 ∥ S5                                                                   | Disjoint file sets (verified 2026-10-05: S3 moves `docs/src/dev/design/check/` + `config.js`; S4 moves `docs/src/rfc/` + `check_tracking.py` generator; S5 moves `src/frontend/core/typecheck/` two `pub`s; no intersection between any two) |

**Not parallel (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5 same batch
file); 4.3.2 ∥ 9.2 (unified to P4); **3.4 ∥ P3.5** (`checker.rs` / `environment.rs` same file —
external track's pub deletion batch and silent channel fix must be serial).

**Documentation internal new constraints**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial; 7a→7b→7c→7d
strictly serial, and 7b's `Phi` arm must precede 7d.

## Key Decisions and Rationale

- **P0 as independent phase rather than merged into P9** — gates are "post-hoc check", rules are
  "pre-judgment". P9's scripts cannot prevent "should refactor but patched", only D0–D4's review
  checklist can.
- **P9 as the only authoritative list of CI scripts** — the three parallel lists from original gap
  G3 couldn't converge. After this table merges 10 scripts each belonging to one phase, report-only
  and hardening timing explicitly queryable.
- **P3.5 absorbs RFC-029g rather than leaving it as "independent track, do after refactor"** — its
  deletion surface coincides with P4/P5/P6/P7/P8's rewrite surface (RFC-039 D56). Leaving for
  post-refactor execution equals redoing the same batch of deletions on new grammar, new checker
  layout, and new IR; this is the same lesson as 7e absorbing `2315`/`2341`, S4 absorbing RFC status
  correction.
- **P5's steps are supplemented in this document rather than merged into P4** — P4 is already the
  largest single point risk, layering checker split exceeds rollback granularity.
- **5.3.2 takes "zero caller change" as hard acceptance** — split must be independently revertable.

## Known Limitations and Risks

- **1.3.x (fix exposed defects) has no regression criterion** — only says "fixed defects should not
  be rolled back", lacks "what was fixed" record requirement. **Execution convention**: each 1.3.x
  fix's commit message must reference fixed test name and defect description, as post-hoc bisection
  basis.
- ~~**7b.1 granularity too coarse**~~ **Fixed (2026-10-05)**: per DoD 6 split into 7b.1a / 7b.1b two
  exclusive commits.
- **8.2.1 granularity too coarse** — about 89 rename references, if missed **silently change
  associativity**.
- ~~**D17 / D20 have no corresponding level-3 task**~~ **Fixed (2026-10-05)**: D17 landed as 7e.1,
  D20 landed as 4.5.1/4.5.2; D32/D34's opcode reclaim merged into 7e.1, deletion action left to P10
  S6.
- ~~**`2315` / `2341` data loss defects originally "independent issue"**~~ **Fixed (2026-10-05)**:
  absorbed as 7e.2 / 7e.3. Exception table (throw/try) is language core semantics, must not be left
  as implementation legacy.
- ~~**E2 (`lib.rs:568/582` `todo!()`) has no phase attribution**~~ **Fixed (2026-10-05)**: merged
  into P10 S2.
- ~~**F7 (opcode gate extractor) attribution ambiguous**~~ **Fixed (2026-10-05)**: merged into P10
  S6's generation-time gate task.

## Conflict Register: All Resolved

> **The original "Conflict Register" C1–C8 and each document's open questions have all converged to
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).** **This
> document does not retain any pending items. Item-by-item correspondence:**

| Original # | Topic                                                        | Decision                                                                                                                                                |
| ---------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1         | `precedence_inline.rs` revived at P1, deleted at P8          | **Revive** (P1's prerequisite is reviving tests with real assertions), P8's 8.6 deletes this file along with Pratt                                      |
| C2         | "Grammar-driven" commitment vs lookup-table-ized             | **Done completely**: adopt LALRPOP, see `05` "Syntax: Complete Grammar-Driven"                                                                          |
| C3         | C6 category definition conflicts with S1 classification      | **S1 classified into C6** (revival doesn't need equivalence criterion, only regression test); C6 definition extended to "pure deletion or pure revival" |
| C4         | `test_release_plan_spans_consumed` whitelist                 | **No whitelist**, difference set must be empty; if not empty indicates `ReleasePlan` contract has defect → change to `PlanId` (D20/D41)                 |
| C5         | `Aggregation` two function internal differences not verified | **P4's 4.2.7 lists it as must-verify item**, verify line by line during implementation                                                                  |
| C6         | wasm playground fix needs P4's `ProgramKind`                 | **wasm path's vulnerability fix merged into 4.2.8**; P3 only handles three native entries                                                               |
| C7         | `method_def_ordinals` attribution                            | **Belongs to 04** (P7 batch c), see D21                                                                                                                 |
| C8         | `upvalue_count` fix and `.42` version number                 | **Fix + `VERSION` upgrade from 4 to 5** (format header has `MAGIC` + `VERSION` field, `codegen/bytecode.rs:14-16`), see D17                             |

## See Also

- [RFC-039 Body](../../rfc/accepted/039-compiler-architecture.md) — phase sequence, DoD, global
  acceptance gates G1–G10's only authority
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 criterion category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0's detailed design and D0–D4
  decision procedure
- [01-routing.md](01-routing.md) — target directory structure (form after task completion)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — each phase's design basis
