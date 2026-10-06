# Multi-level Construction Task Table (WBS)

> **Companion design document**. This document is a companion to
> [RFC-039: Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). It
> expands the RFC's stage sequence into a **three-level task breakdown that can be committed and
> verified independently**.
>
> The stage order and acceptance gates are solely authoritative in RFC-039; this document is
> responsible only for breaking them down to an executable granularity and logging unresolved
> conflicts.

## Position and Scope

| Level       | Count       | Meaning                                                                 |
| ----------- | ----------- | ----------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Stages, in one-to-one correspondence with the stage sequence in RFC-039 |
| **Level 2** | 45          | Task groups, drawn from each document's "Implementation Points"         |
| **Level 3** | 122         | Independently committable actions                                       |

**This document covers**: task breakdown, dependencies, parallel groups, and binding of acceptance
criteria.

**This document does not cover**: the design rationale for each task (see `02`–`08`), nor the
justification of stage ordering (see RFC-039).

## Current Status: Gaps Discovered During Breakdown

While consolidating the "Implementation Points" from 8 documents into this table, **5 hard gaps in
the documentation system itself were found**. All have been resolved:

| #   | Gap                                                                                                                                                                                                                                                                                                                                                                  | Resolution                                                                                                                                                          |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no construction steps at all**. The RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents has a section describing how this split is done; the directory tree in `01-routing.md` mentions `annotations.rs` in one line, the only mention across the whole batch                                                         | P5 steps are filled in by this document (see below)                                                                                                                 |
| G2  | **`include!` refactor: claimed by three documents, landed nowhere**. The RFC lists it under P4; `02`'s "see also" assigns it to itself; `06`'s S7 also lists it. But `02`'s change list (6 new + 20 modified) **does not include `semantic_tokens.rs`**                                                                                                              | Construction steps are assigned to §P5 5.1 of this document (after P4 is complete); references in the RFC and 06 now point here                                     |
| G3  | **Three coexisting lists of CI scripts**. The RFC lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists another 4 checks (with entirely different names); `04` additionally requires `check-synth-boundary.py`. After deduplication, **at least 9 new scripts, with no unified list**; `check-obligations.py` is doubly assigned to P4 and P9 | The unified list is in §P9 of this document (10 scripts, each owned by exactly one stage); all other documents reference this table                                 |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it among the "1005 lines that need resurrection", and the same document's B6 says it must be deleted along with the `Precedence` enum; `05` argues for deletion                                                                                                                                      | **Resurrect in P1, delete with Pratt in P8** (logged as decision C1) — first let CI regain coverage; the deletion happens together with the dead ladder in P8's 8.6 |
| G5  | **L2 goal statement conflict**. The RFC and this directory's index say "grammar-driven syntax", but `05` self-assesses as "**did not adopt a true grammar-driven approach (LALRPOP); only achieved declarative table lookup**"                                                                                                                                       | Resolved to **full LALRPOP grammar-driven** (`05` §Syntax: Full Grammar-Driven); G7/G8 acceptance gates execute against this commitment                             |

## Target Design: Stage Sequence

```
P0  Repository maintenance mechanism and code placement rules      08        ← Newly added, establish the rules first
P1  Resurrect orphan tests and fix defects                         06 §S1
P2  Establish the equivalence criterion baseline                   07 all
P3  Fix correctness vulnerabilities (minimum scheme)               02 §S3 vulnerability half
P4  Stage contract and unified Driver                              02
P5  In-file checker split                                          02 + supplemented by this document
P6  Type representation unification                                03
P7  Intermediate representation SSA conversion                     04      } parallelizable
P8  Frontend paradigm change                                       05      }
P9  Anti-rebound gates (with unified script list)                  01 §Anti-rebound + 08 §Machine-checkable rules
P10 Remaining cleanup and status corrections                       06 §S2/S3/S4/S5/S6
```

**Rationale for adding P0 as a separate stage**: Three prohibitions (no fabrication / no endless
padding / patching where a refactor is needed) constrain every action from P1 to P10. The rules must
be in place before work begins; otherwise, by the time P1 resurrects tests, new `mod` declarations
are already being added to `tests/mod.rs` — which is exactly the cause of the G4 conflict.

## Detailed Design: Three-level Task Table

### P0 Repository Maintenance Mechanism and Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

- [ ] **0.1 Rules written up** (3 level-3 tasks; touchpoints 0.1.2 and rule extraction 0.1.3 were
      landed in advance on 2026-10-05, leaving 0.1.1 CONTRIBUTING main text review)
- [ ] **0.2 Gate implementation** (3 level-3 tasks)
- [ ] **0.3 Table-gate extension** (1 level-3 task)

| Level 2                  | Level 3                                                                                                                                                                                                                                                                                                                                                               | Prerequisite | Acceptance                                                                                                                                                                                                  |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Rules written up     | 0.1.1 `CONTRIBUTING.md` writes D0–D4 and the three prohibitions; the responsibility categories reference the directory-responsibility table in `01-routing.md`                                                                                                                                                                                                        | —            | Manual review passes                                                                                                                                                                                        |
|                          | 0.1.2 Implementer touchpoints landed (**completed 2026-10-05**): create `AGENTS.md` (repository root entry) and `compiler-architecture/HOWTO.md` (pre-work self-check + D3 patch judgment); add a "Responsibility ownership and decision procedure" required block to the PR template (D44); add a "Code placement and change procedure" section to `CONTRIBUTING.md` | —            | Landed; review verifies the four touchpoints                                                                                                                                                                |
|                          | 0.1.3 Rule-body extraction (**completed 2026-10-05**): create `docs/src/dev/coding-rules.md` (precise rule descriptions, long-term validity, not archived with RFC-039); add a migration preamble to 08 turning it into a diagnostic record; all four touchpoints now point to coding-rules                                                                           | —            | Landed                                                                                                                                                                                                      |
| 0.2 Gate implementation  | 0.2.1 `check-concepts.py` (Prohibition 1 A/B/C/D)                                                                                                                                                                                                                                                                                                                     | 0.1.1        | **Must, on unmodified code, report 3 sets of operator enums and 2 sets of parallel type representations (counting the `ir::Type` alias as well)**; report-only at this stage, P9 close-out switches to hard |
|                          | 0.2.2 `check-fanout.py` (Prohibition 3 A/B/C)                                                                                                                                                                                                                                                                                                                         | 0.1.1        | **Intentionally adding a 6th entry-style wiring must turn red**; report-only at this stage, P9 close-out switches to hard                                                                                   |
|                          | 0.2.3 `check-boundary.py` (Prohibition 2 criterion C)                                                                                                                                                                                                                                                                                                                 | 0.1.1        | **Intentionally adding one L2→L3 reverse `use` must turn red**; record the initial count of `pub(crate)` leakages                                                                                           |
| 0.3 Table-gate extension | 0.3.1 Extend `tools/code-tables` to cover opcodes and type tables                                                                                                                                                                                                                                                                                                     | 0.1.1        | Depends on P6's T1; closes out independently                                                                                                                                                                |

> **No scale-baseline task**. The 2026-10-03 decision cancels all line-count/volume gates; scale
> issues are resolved through responsibility separation (see `08` Prohibition 2 criterion A, manual
> judgment). **0.2.1 is the core acceptance of this stage**: if the gate cannot report the known
> parallel representations, the criterion is mis-designed — this is a false gate.

### P1 Resurrect Orphan Tests and Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Points S1 + [05](05-frontend-paradigm.md)
§Test Reconstruction

- [ ] **1.1 Resurrect `lexer/tests/`** (4 level-3 tasks)
- [ ] **1.2 Resurrect 4 locations in live directories** (1 level-3 task)
- [ ] **1.3 Fix defects surfaced** (2 level-3 tasks)

| Level 2                                       | Level 3                                                                                                                                                    | File:Line                  | Prerequisite | Acceptance                                                            |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | --------------------------------------------------------------------- |
| 1.1 Resurrect `lexer/tests/`                  | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                        | After `lexer/mod.rs:106`   | —            | Test count rises                                                      |
|                                               | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                                | `lexer/tests/mod.rs:15-25` | 1.1.1        | **Without this, 159 lines still do not run**                          |
|                                               | 1.1.3 Delete 7 empty shell files + their `mod` declarations                                                                                                | 7 files in `lexer/tests/`  | 1.1.1        | C6 has no criterion                                                   |
|                                               | 1.1.4 Delete the `pub use ...::*;` re-export block                                                                                                         | `tests/mod.rs:28-38`       | 1.1.3        | That block is exactly what was masking the empty shells               |
| 1.2 Resurrect 4 locations in live directories | 1.2.1–1.2.4 wire up `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` respectively | Each parent module         | —            | Test count rises; **1.2.1 conflicts with 8.0.1, see conflict log C1** |
| 1.3 Fix defects surfaced                      | 1.3.1 Four radix scanner overflow paths                                                                                                                    | `literals.rs`              | 1.1.x        | `cargo test` green                                                    |
|                                               | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                       | `literals.rs`              | 1.1.x        | Same as above                                                         |

**Acceptance**: test count rises; **real defects are expected to surface, so leave room for fix
time**. Revert file-by-file, but **the defects fixed should not be rolled back**.

### P2 Establish the Equivalence Criterion Baseline

Source: [07](07-equivalence-oracle.md) all (the document has no "Implementation Points" section;
reverse-engineered from structure)

- [ ] **2.1 IR static verifier** (3 level-3 tasks)
- [ ] **2.2 Normalized snapshots** (2 level-3 tasks)
- [ ] **2.3 Corpus differential** (3 level-3 tasks)
- [ ] **2.4 Vulnerability-specific criteria** (3 level-3 tasks)
- [ ] **2.5 Regression gates** (1 level-3 task)

| Level 2                             | Level 3                                                                                                                                                         | Prerequisite | Acceptance                                                                                 |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------ |
| 2.1 IR static verifier              | 2.1.1 `verify.rs` with dual modes `verify_loose` / `verify_ssa`                                                                                                 | —            | C4                                                                                         |
|                                     | 2.1.2 **`verify_loose` runs green on the existing non-SSA IR**                                                                                                  | 2.1.1        | **Hard threshold, admission condition for P7 batch a**                                     |
|                                     | 2.1.3 Implement 7 invariants (dominance / single definition / Phi consistency / jump targets / global out-of-bounds / type consistency / inner-scope isolation) | 2.1.1        | C4                                                                                         |
| 2.2 Normalized snapshots            | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / sort predecessors)                                                               | —            | C1/C3/C5                                                                                   |
|                                     | 2.2.2 Snapshot ingestion + manual review flow (`src/middle/core/tests/snapshots/`)                                                                              | 2.2.1        | C1/C3/C5                                                                                   |
| 2.3 Corpus differential             | 2.3.1 Differential framework + 293-corpus baseline                                                                                                              | —            | Diagnostics / exit code / stdout per item                                                  |
|                                     | 2.3.2 **Multifile corpus layer** (create `tests/yaoxiang-multifile/`, with `yaoxiang.toml` project fixtures — decision D48)                                     | —            | **Unconditionally required (D40); the only executable behavioral-criterion source for P4** |
|                                     | 2.3.3 C4 behavioral differential mandatory-coverage list (10 semantic categories) + performance baseline (criterion smoke benchmarks)                           | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                                     |
| 2.4 Vulnerability-specific criteria | 2.4.1 `test_multifile_proof_obligation_not_dropped`                                                                                                             | 2.3.2        | **Must turn red first**                                                                    |
|                                     | 2.4.3 `test_no_silent_pass_on_unproven`                                                                                                                         | —            | Catches `checker.rs:5179/5318/5448`                                                        |
|                                     | 2.4.4 `test_release_plan_spans_consumed`                                                                                                                        | —            | **No whitelist; the diff set must be empty (D41)**                                         |
| 2.5 Regression gates                | 2.5.1–2.5.4 see the P9 unified list                                                                                                                             | 2.1–2.3      | See P9                                                                                     |

> The original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> are **moved out of P2**: the `Program` / `Obligations` types they reference do not exist until P4,
> and P2 cannot compile them. The former becomes P4's 4.1.4 (avoid double registration); the
> latter's `#[ignore]` red skeleton merges into 4.3.1.

### P3 Fix Correctness Vulnerabilities (Minimum Scheme)

Source: [02](02-stage-contract.md) §Implementation Points S3, vulnerability-fix half

- [ ] **3.1 `proof_calls` consumer side** (2 level-3 tasks)
- [ ] **3.2 The second silent-drop point** (1 level-3 task)
- [ ] **3.3 panic-to-diagnostic** (1 level-3 task)

| Level 2                          | Level 3                                                | File:Line                                | Prerequisite | Acceptance                          |
| -------------------------------- | ------------------------------------------------------ | ---------------------------------------- | ------------ | ----------------------------------- |
| 3.1 `proof_calls` consumer side  | 3.1.1 Make `proof_calls` private + sole getter         | `types.rs:29`                            | 2.4.1        | Vulnerability criterion turns green |
|                                  | 3.1.2 Fill in the three consumer sites                 | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1        | C2                                  |
| 3.2 The second silent-drop point | 3.2.1 `Unproven` empty match arm produces a diagnostic | `checker.rs:1303-1314`                   | 2.4.3        | `test_no_silent_pass_on_unproven`   |
| 3.3 panic-to-diagnostic          | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic  | `predicate.rs:34-36`                     | —            | No panic when Z3 is missing         |

**Acceptance**: vulnerability criteria turn green, and **intentionally removing the fix must make
them turn red again**. **Risk**: adding E4018 on the multifile path is a breaking change.

> **Stop-bleeding channel (2026-10-05 ruling)**: P3 does not have to wait for P2 to finish in full.
> The only hard prerequisites for 3.1 / 3.2 are **2.3.2 (multifile corpus layer) + 2.4.1 / 2.4.3
> (two red criteria)**; 2.1 / 2.2 / 2.3.1 / 2.3.3 (IR verifier, snapshots, single-file differential,
> performance baseline) can proceed in parallel with P3. Stopping the bleed on the correctness
> vulnerability (`Sorted(3)` silent pass) should not be blocked by snapshot-infrastructure
> construction. 3.3 (panic-to-diagnostic) has no prerequisite and can land any time.

### P4 Stage Contract and Unified Driver

Source: [02](02-stage-contract.md) §Implementation Points S1–S5. **The fix for this stage and P3
must be split into two commits.**

- [ ] **4.1 Declarative stage table** (5 level-3 tasks)
- [ ] **4.2 Entry consolidation** (8 level-3 tasks)
- [ ] **4.3 Obligation ledger** (2 level-3 tasks)
- [ ] **4.4 Proof layer and wasm wrap-up** (3 level-3 tasks)
- [ ] **4.5 Cross-layer contract PlanId-ification (D20)** (2 level-3 tasks)

| Level 2                                         | Level 3                                                                                                                                                     | Prerequisite | Acceptance                                                                                                                                                             |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative stage table                     | 4.1.1 `stage.rs` (`Stage` variants + `Stage::ALL` + `StageScope`)                                                                                           | P0           | `test_program_stage_coverage`                                                                                                                                          |
|                                                 | 4.1.2 `program.rs` / `unit.rs`                                                                                                                              | 4.1.1        | Same as above                                                                                                                                                          |
|                                                 | 4.1.3 `Pipeline::run` switched to Driver (`pipeline.rs:141-227`)                                                                                            | 4.1.2        | **Single-file diagnostic set and exit code byte-for-byte identical**                                                                                                   |
|                                                 | 4.1.4 Stage-coverage assertion                                                                                                                              | 4.1.3        | C2                                                                                                                                                                     |
|                                                 | 4.1.5 Zero-diff across the full corpus                                                                                                                      | 4.1.4        | Diagnostics / exit code / stdout all zero-diff (293 corpora)                                                                                                           |
| 4.2 Entry consolidation                         | 4.2.1 `compile_project` slimmed down to a Program constructor (`orchestrator.rs:99-237`)                                                                    | 4.1.5        | Diagnostic sets of the four entries identical after unification                                                                                                        |
|                                                 | 4.2.2 `check_project` (`:273-399`)                                                                                                                          | 4.2.1        | Same as above                                                                                                                                                          |
|                                                 | 4.2.3 `check_source_in_project` (`:450`)                                                                                                                    | 4.2.2        | Same as above                                                                                                                                                          |
|                                                 | 4.2.4 `compile_embedded_module` (`:1374`)                                                                                                                   | 4.2.3        | Same as above                                                                                                                                                          |
|                                                 | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                                              | 4.2.1        | `check` is consistent inside and outside the project                                                                                                                   |
|                                                 | 4.2.6 Delete the LSP manual stage sequence (`lsp/handlers/diagnostics.rs:161-227`)                                                                          | 4.2.1        | LSP and CLI diagnostic sets identical                                                                                                                                  |
|                                                 | 4.2.7 `Aggregation` parameter-driven two-way choice (`orchestrator.rs:486-494`)                                                                             | 4.2.1        | **Line-by-line verification of internal differences between the two functions during implementation** (conflict log C5 has been ruled as a required verification item) |
|                                                 | 4.2.8 wasm goes through `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)                                                                       | 4.2.1        | See C6                                                                                                                                                                 |
| 4.3 Obligation ledger                           | 4.3.1 `obligations.rs` + `assert_drained()` (newly created; includes the `#[ignore]` red skeleton first)                                                    | 4.1.3        | `test_obligations_drained` turns green                                                                                                                                 |
|                                                 | 4.3.2 Obligation-diagnostic W→E upgrade                                                                                                                     | 4.3.1        | Manual review of each newly added E                                                                                                                                    |
| 4.4 Proof layer and wasm wrap-up                | 4.4.1 `layers/README.md` layer-order change to actual order                                                                                                 | —            | **Runs independently of 4.1–4.3**                                                                                                                                      |
|                                                 | 4.4.2 `default_solver()` changed to a `&'static` singleton (`proof/smt/backend.rs:67-72`)                                                                   | —            | Cache hit rate is observable                                                                                                                                           |
|                                                 | 4.4.3 Degraded path adds a warning (`checker.rs:1283-1293`)                                                                                                 | —            | —                                                                                                                                                                      |
| 4.5 Cross-layer contract PlanId-ification (D20) | 4.5.1 `ReleasePlan` key `Span` → `PlanId` (allocated on the producer side at `layers/ownership.rs:31` and matched on the consumer side at `ir_gen.rs:1942`) | 4.3.1        | `test_release_plan_spans_consumed` diff set is empty (D41)                                                                                                             |
|                                                 | 4.5.2 `overload_resolutions` key `Span` → `PlanId`                                                                                                          | 4.5.1        | C2; span-mismatch-type silent failures drop to zero                                                                                                                    |

### P5 In-File Checker Split — Filled in by this document (original gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (the `pub(crate)` path of
`collect_used_in_type` must not change)

- [ ] **5.1 `include!` → real `mod`** (2 level-3 tasks)
- [ ] **5.2 Extract `refinement`** (2 level-3 tasks)
- [ ] **5.3 Extract `annotations`** (2 level-3 tasks)

| Level 2                     | Level 3                                                                                                                                                                                                                                                                          | Prerequisite | Acceptance                                                       |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ---------------------------------------------------------------- |
| 5.1 `include!` → real `mod` | 5.1.1 The `include!` at `checker.rs:5618` becomes a real `mod` declaration                                                                                                                                                                                                       | P4 complete  | C1 zero-diff; `semantic_tokens.rs` acquires module identity      |
|                             | 5.1.2 Add `use super::*` or per-item imports (the `include!` era inherited scope implicitly)                                                                                                                                                                                     | 5.1.1        | `cargo build` green                                              |
| 5.2 Extract `refinement`    | 5.2.1 Move out the refinement block (current `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                                                | 5.1.1        | C1 zero-diff                                                     |
|                             | 5.2.2 `collect_refined_binding_checks` signature change: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                                             | 5.2.1        | C1; the only call site is `1235`                                 |
| 5.3 Extract `annotations`   | 5.3.1 Move out annotation validation (current `3441-3661`, including `is_predicate_head`)                                                                                                                                                                                        | 5.2.2        | C1 zero-diff                                                     |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) stays `pub(crate)` and remains under the `checker` module path** → add `pub(crate) use annotations::*;` at the top of `checker.rs`, so the import at `inference/statements.rs:16` is **unchanged character-for-character** | 5.3.1        | **Zero caller-side changes** is the hard acceptance of this step |

**Hard constraint**: `collect_used_in_type` is used cross-module at `inference/statements.rs:16`.
The approach is to `pub(crate) use` re-export, making the split a **pure relocation**.

**Only split proven large blocks.** Do not split modules of only 200–400 lines — that increases
navigation cost, reduces cohesion, and yields zero decoupling benefit. The remaining sub-modules
(signatures / type_defs / imports) wait until the file grows back to 1,500 lines.

### P6 Type Representation Unification

Source: [03](03-type-unification.md) §Implementation Points stages 0–6, criterion C3. **Stages 0→6
are strictly serial.**

- [ ] **6.0 Gates first** (2 level-3 tasks)
- [ ] **6.1 Name normalization** (3 level-3 tasks)
- [ ] **6.2 Bytecode type unification** (3 level-3 tasks)
- [ ] **6.3 Dead-variant deletion** (2 level-3 tasks)
- [ ] **6.4 Parser data flow** (5 level-3 tasks)
- [ ] **6.5 AST dead variants** (2 level-3 tasks)
- [ ] **6.6 Gate hardens** (1 level-3 task)
- [ ] **6.7 Directory rename (D1, pure relocation batch)** (1 level-3 task)
- [ ] **6.8 Annotation check mode and Fn parameter representation (D54, added 2026-10-05)** (2
      level-3 tasks)

| Level 2                                                         | Level 3                                                                                                                                                                                                                                                                                                                                                                                                                        | File:Line                                                                     | Acceptance                                                                                                                                                                                                                                     |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.0 Gates first                                                 | 6.0.1 Create `tools/type-tables`, implement T1/T2/T3, **T1 first reports only, does not fail**                                                                                                                                                                                                                                                                                                                                 | New crate                                                                     | T1 accurately lists 13 forward zero-construction variants **+ lists reverse-bridge reconstruction points separately**                                                                                                                          |
|                                                                 | 6.0.2 Take a full baseline on unmodified code                                                                                                                                                                                                                                                                                                                                                                                  | —                                                                             | No code changes                                                                                                                                                                                                                                |
| 6.1 Name normalization                                          | 6.1.1 Count occurrences of non-canonical type names in the corpus                                                                                                                                                                                                                                                                                                                                                              | `tests/yaoxiang/`                                                             | Determines whether synonyms can be deleted directly                                                                                                                                                                                            |
|                                                                 | 6.1.2 `from_builtin_name` cuts synonyms → lexer alias table                                                                                                                                                                                                                                                                                                                                                                    | `mono.rs:618-643`                                                             | C3                                                                                                                                                                                                                                             |
|                                                                 | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                                                                                                                                                                                                                                           | `types.rs:162-165`                                                            | **Must be verified line-by-line; cannot assume equivalence**                                                                                                                                                                                   |
| 6.2 Bytecode type unification                                   | 6.2.1 Three sites change to `MonoType`                                                                                                                                                                                                                                                                                                                                                                                         | `bytecode.rs:812/814/854`, `image.rs:44`                                      | C3 + targeted `dump_bytecode` cross-comparison                                                                                                                                                                                                 |
|                                                                 | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                                                                                                                                                                                                                                       | `bytecode.rs:2352-2390`                                                       | Same as above                                                                                                                                                                                                                                  |
|                                                                 | 6.2.3 `type_table` element type changed to `MonoType`                                                                                                                                                                                                                                                                                                                                                                          | `image.rs:44`                                                                 | Same as above                                                                                                                                                                                                                                  |
| 6.3 Dead-variant deletion                                       | 6.3.1 Delete 11 variants + clear match arms + fallback                                                                                                                                                                                                                                                                                                                                                                         | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                                   | C3 + `cargo build` green                                                                                                                                                                                                                       |
|                                                                 | 6.3.2 **Must land in the same batch as 6.3.1**; rewrite the reverse bridge                                                                                                                                                                                                                                                                                                                                                     | `passes/mono/function.rs:507-529` (`mono_to_ast_type` reconstructs 7 of them) | Generic-substitution targeted cross-comparison, otherwise **silently changes behavior**                                                                                                                                                        |
| 6.4 Parser data flow                                            | 6.4.1 Add `probe.rs`                                                                                                                                                                                                                                                                                                                                                                                                           | New                                                                           | C3                                                                                                                                                                                                                                             |
|                                                                 | 6.4.2 Modify `parser_state.rs:46-58`                                                                                                                                                                                                                                                                                                                                                                                           | —                                                                             | Same as above                                                                                                                                                                                                                                  |
|                                                                 | 6.4.3 Delete calls to `operator_interfaces::spec()` (**2 sites**: `declarations.rs:509`, `ast.rs:930`)                                                                                                                                                                                                                                                                                                                         | —                                                                             | Same as above                                                                                                                                                                                                                                  |
|                                                                 | 6.4.4 Merge two `name_used_as_type*`                                                                                                                                                                                                                                                                                                                                                                                           | `ast.rs:847` + `declarations.rs:42`                                           | Same as above                                                                                                                                                                                                                                  |
|                                                                 | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) wired through `NameKind::Builtin`                                                                                                                                                                                                                                                                                                                                                     | —                                                                             | Same as above                                                                                                                                                                                                                                  |
| 6.5 AST dead variants                                           | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                                                                                                                                                                                                                                                     | `ast.rs:37-43` + 4 consumer sites + 2 exhaustiveness arms                     | `tests/integration/` 18 modules green (**split into 2 commits**)                                                                                                                                                                               |
|                                                                 | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                                                                                                                                                                                                                                                | `ast.rs:241-249`                                                              | Marked **semantic risk**                                                                                                                                                                                                                       |
| 6.6 Gate hardens                                                | 6.6.1 T1 changed from warning to `panic!`                                                                                                                                                                                                                                                                                                                                                                                      | `tools/type-tables`                                                           | **Intentionally introducing a zero-construction variant must turn red**                                                                                                                                                                        |
| 6.7 Directory rename (D1, pure relocation batch)                | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; top-level `parser/ast.rs` split into `ast/`; bytecode-domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` moved to `middle/bytecode/`                                                                                                                                                                                                                        | Whole repo                                                                    | **C1 snapshot zero-diff**; dedicated commit; out-of-scope `use` baseline does not regress                                                                                                                                                      |
| 6.8 Annotation check mode and Fn parameter representation (D54) | 6.8.1 `Type::Fn.params: Vec<Type>` → `Vec<Param>` (name and type co-resident), closed in the same batch as 6.5.2's deletion of `Assign.signature_params` — 6.5.2 only deletes the old representation, this item provides the replacement                                                                                                                                                                                       | `ast.rs:451-454`, ~20 consumer files (mechanical change)                      | C3 (pure representation reshape batch, compared to the not-yet-enabled state); split into a dedicated commit                                                                                                                                   |
|                                                                 | 6.8.2 Declaration-driven check mode: parameter types positionally drive the lambda header (name-independent), the output (tail expression and return) uniformly checks against the declared return type, non-lambda values are `infer+unify` (isomorphic to `m: Int = "hello"`); failing criteria first — u1–u4/v1–v5 probes added to the corpus (red first, then green); delete `declarations.rs:337-376` parser name merging | `statements.rs:1170-1914`                                                     | The compared-to-enabled batch is a **behavioral fix**: new diagnostic codes, baseline update; C3 not applicable; v1–v5 share the same judgment across five syntactic forms (eliminating "same semantics, different syntax, one red one green") |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA Conversion

Source: [04](04-ssa.md) §Implementation Points batches a–d, criterion C4, **strictly serial within
each batch**.

- [ ] **7a Cut multiple definitions** (3 level-3 tasks)
- [ ] **7b SSA shape switch** (6 level-3 tasks; 7b.1 already split into 1a/1b per DoD 6)
- [ ] **7c Make implicit contracts explicit** (3 level-3 tasks)
- [ ] **7d Do last** (2 level-3 tasks)
- [ ] **7e `.42` data-loss fix** (3 level-3 tasks; 2315/2341 absorbed from "independent issue" on
      2026-10-05)

| Level 2                             | Level 3                                                                                                                                              | Prerequisite                        | Acceptance                                         |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------------- |
| 7a Cut multiple definitions         | 7a.1 Delete 3 statement-level reclamations                                                                                                           | **2.1.2 `verify_loose` runs green** | C4                                                 |
|                                     | 7a.2 6 save/restore sites replaced with RAII guards                                                                                                  | 7a.1                                | C1 snapshot zero-diff                              |
|                                     | 7a.3 Caliber unification (`1798` vs `2001`)                                                                                                          | 7a.2                                | C1                                                 |
| 7b SSA shape switch                 | 7b.1a `ir.rs` structural change #1-2 (`Operand` variant consolidation + introduction of `Instruction::Phi`)                                          | 7a.3 + **P6 complete**              | C4; dedicated commit                               |
|                                     | 7b.1b `ir.rs` structural change #3-5 (`FunctionIR` / `BasicBlock` / type-annotation fields)                                                          | 7b.1a                               | C4; dedicated commit                               |
|                                     | 7b.2 `next_temp_reg` changed to `Operand::Value`                                                                                                     | 7b.1                                | Same as above                                      |
|                                     | 7b.3 `translator.rs` adds the `Phi` arm                                                                                                              | 7b.1                                | C4 + `.42` round-trip test                         |
|                                     | 7b.4 Linear-scan allocator (within the u8/255-slot model)                                                                                            | 7b.3                                | Same as above (+500~1000 lines)                    |
| 7c Make implicit contracts explicit | 7c.1 `synth.rs` boundary + boundary-check script                                                                                                     | 7b.4                                | C4 + new script enters CI                          |
|                                     | 7c.2 Span consumption count                                                                                                                          | 7c.1                                | Consumption count is **zero**                      |
|                                     | 7c.3 `method_def_ordinals` made read-only                                                                                                            | 7c.1                                | Ownership unclear (C7)                             |
| 7d Do last                          | 7d.1 `generate_call_expr_ir` split into `CallArgs` + 6 `emit_*`                                                                                      | 7c.x                                | C1 snapshot zero-diff                              |
|                                     | 7d.2 "Fill 0" fallback changed to a return diagnostic                                                                                                | 7d.1                                | C4                                                 |
| 7e `.42` data-loss fix              | 7e.1 `upvalue_count: 0` correction (`bytecode.rs:2312`) + `VERSION` 4→5 (D17) + reclaim allocated-but-unused opcodes (D32/D34 with the version bump) | 7d.x                                | C4 + `.42` round-trip test; dedicated commit       |
|                                     | 7e.2 `exception_handlers` persisted (`bytecode.rs:2315` — throw/try are core language semantics, **must not be left as implementation debt**)        | 7e.1                                | `.42` direct-run exception case differential       |
|                                     | 7e.3 `globals` persisted (`bytecode.rs:2341`)                                                                                                        | 7e.1                                | `.42` direct-run global-variable case differential |

**Expectation management**: **net increase of 900–1600 lines**. If "code gets shorter" is held as
the success criterion at project kickoff, this stage will be judged as a failure.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Points stages 0–4, criterion C5.

- [ ] **8.0 Dead-ladder cleanup (C6)** (2 level-3 tasks)
- [ ] **8.2 Lexical convergence** (2 level-3 tasks)
- [ ] **8.3 Build the LALRPOP grammar** (3 level-3 tasks)
- [ ] **8.4 Dual-parser differential** (1 level-3 task)
- [ ] **8.5 Cut over** (1 level-3 task)
- [ ] **8.6 Delete Pratt** (2 level-3 tasks)
- [ ] **8.7 `parse_assign_after_target` split** (1 level-3 task)
- [ ] **8.8 Test completion** (3 level-3 tasks)
- [ ] **8.9 Parameter-position form unification (D53, added 2026-10-05)** (1 level-3 task)

| Level 2                                       | Level 3                                                                                                                                                                                                                                                                                                                                                       | Prerequisite                     | Acceptance                                                                                                                          |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 8.0 Dead-ladder cleanup (C6)                  | 8.0.1 Delete the `Precedence` enum + `PrecedenceContext` (96 lines)                                                                                                                                                                                                                                                                                           | **Conflicts with 1.2.1, see C1** | C6 has no criterion                                                                                                                 |
|                                               | 8.0.2 Name bare magic numbers (`(6,7)` / `(11,1)` / `12`)                                                                                                                                                                                                                                                                                                     | —                                | Same as above                                                                                                                       |
| 8.2 Lexical convergence                       | 8.2.1 Merge four radix scanners + merge three escape sites + merge multi-line strings                                                                                                                                                                                                                                                                         | —                                | C5; `literals.rs` shrinks by ~500 lines                                                                                             |
|                                               | 8.2.2 Eliminate f-string nested compilation                                                                                                                                                                                                                                                                                                                   | 8.2.1                            | **Span must change**, so the baseline must be taken first                                                                           |
| 8.3 Build the LALRPOP grammar                 | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                                                                                                                                                                                                                                                                                 | 8.2.x                            | `cargo build` passes; produces AST for the 293 corpora (**no diff**)                                                                |
|                                               | 8.3.2 Action code `grammar/actions.rs` (one function per `Expr` variant, 22 in total)                                                                                                                                                                                                                                                                         | 8.3.1                            | Same as above                                                                                                                       |
|                                               | 8.3.3 Error productions (preserve `Expr::Error` / `StmtKind::Error` placeholder semantics)                                                                                                                                                                                                                                                                    | 8.3.2                            | Same as above                                                                                                                       |
| 8.4 **Dual-parser differential**              | 8.4.1 Both parsers run the 293 corpora + `src/std/tests`; after AST normalization, **bit-by-bit comparison**                                                                                                                                                                                                                                                  | 8.3.3                            | **AST fully equivalent for legal programs — this is the equivalence proof for the entire grammar migration**                        |
| 8.5 Cut over                                  | 8.5.1 `parse()` calls LALRPOP; Pratt remains as `parse_legacy()`                                                                                                                                                                                                                                                                                              | 8.4.1                            | Full corpus behaviorally equivalent + diagnostic code+span item-by-item identical (**no relaxation**, no C5′)                       |
| 8.6 Delete Pratt                              | 8.6.1 Delete `nud.rs` (1326) + `led.rs` (451) + two sets of BP ladders + ~89 references                                                                                                                                                                                                                                                                       | 8.5.1                            | Full corpus green; `git grep BP_` zero hits                                                                                         |
|                                               | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally dead)                                                                                                                                                                                                                                                                                              | 8.6.1                            | Same as above                                                                                                                       |
| 8.7 `parse_assign_after_target` split         | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                                                                                                                                                                                                                                                                                        | 8.6.x                            | That function only dispatches                                                                                                       |
| 8.8 Test completion                           | 8.8.1 Add associativity cases (**red first, then green**)                                                                                                                                                                                                                                                                                                     | 8.2.x                            | C5                                                                                                                                  |
|                                               | 8.8.2 Add four categories of literal error paths                                                                                                                                                                                                                                                                                                              | —                                | Cover untested paths                                                                                                                |
|                                               | 8.8.3 Add `pratt/tests/mod.rs` wiring self-check assertion                                                                                                                                                                                                                                                                                                    | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6; switch to `parser/tests/`**)                                |
| 8.9 Parameter-position form unification (D53) | 8.9.1 Grammar production `Param ::= Identifier ':' TypeExpr \| TypeExpr`: bare identifiers are resolved in the type namespace, report E if unresolvable; unnamed signatures require the lambda header to carry parameter names (existing rule from RFC-007:47); the form table and interface example (`(Surface)`) from RFC-010 are reconciled with the batch | 8.3.2 + 6.8.1                    | Corpus survey finds zero migrations (all bare identifiers in std/corpora are real types); unknown-type identifiers get a new E code |

**Risk**: the 8.4 dual-parser differential is a **non-skippable equivalence gate**. If it surfaces
non-equivalence, it must be resolved before 8.5 cuts over, not papered over by a hypothetical C5′
afterward. The ~89 BP reference migrations in 8.6, if missed, **silently change associativity** (no
error, only the parse result changes).

> **8.3–8.6 depend on `08`'s D0/D2**: if the grammar file is placed outside `parser/`, or if the
> action code reverse-references `sema`, it will be caught at `check-boundary.py`.

### P9 Anti-Rebound Gates (Unified Script List — Filled in by this document, original gap G3)

Source: [01](01-routing.md) §Anti-rebound mechanism + [08](08-maintenance-mechanism.md)
§Machine-checkable rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts enter CI in their
"introduction stage"; report-only ones are switched to hard in P9 (changed from "report" to "fail")
— existing violations (parallel representations, disambiguation aliases, etc.) only clear out by
P6/P8; switching to hard prematurely would keep CI red long-term and let gates be bypassed manually,
so when P9 switches to hard, the gates should be exactly all-green. P2's 2.5, P4's 4.3.2, and P7's
7c.1 all reference this table and do not establish separate lists.

- [ ] **9.1–9.10 the ten scripts enter CI in their introduction stages; P9 uniformly switches them
      to hard** (details in the table below)

| #    | Script                    | What it checks                                                                                                                      | Introduction stage                              | Acceptance                                                  |
| ---- | ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------- |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` bidirectional assertion                                                                          | P4                                              | Intentionally omitting one stage must turn red              |
| 9.2  | `check-obligations.py`    | A field appears ≥2 times (defined + written) but no third file reads it                                                             | P4 (**sole owner, no longer duplicated in P9**) | Intentionally leaving one non-consumed field must turn red  |
| 9.3  | `check-boundary.py`       | Forbids `include!`; `pub(crate)` cross-layer leakage only allowed to decrease; forbids L2→L3 (**sole owner**, defined by `08`'s P0) | **P0**                                          | Intentionally adding `include!` must turn red               |
| 9.4  | `check-test-wiring.py`    | Has a `tests/` directory but the parent module lacks `mod tests;` → fail                                                            | P1                                              | Intentionally creating an orphan directory must turn red    |
| 9.5  | `check-snapshot-drift.sh` | Snapshot diff exists but the commit message lacks a `snapshot-update` marker                                                        | P2                                              | Same as above                                               |
| 9.6  | `check-ir-verifier.sh`    | Non-empty `verify_loose` across the full corpus                                                                                     | P2                                              | Same as above                                               |
| 9.7  | `check-corpus-parity.py`  | Non-empty corpus differential (diagnostic code+span item-by-item comparison, **no group-based relaxation whatsoever**)              | P2                                              | Same as above                                               |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction appearing outside `synth.rs`                                                                               | P7                                              | Same as above                                               |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representations, disambiguation aliases, synonym table)                                             | **P0** (report-only, switches to hard in P9)    | Must, on unmodified code, report 3 sets of operator enums   |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                                 | **P0**                                          | Intentionally adding a 6th entry-style wiring must turn red |

> **No line-count ratchet script** (`check-file-size.py` + `baseline.toml` were cancelled on
> 2026-10-03). Scale issues are resolved through `08` Prohibition 2 criterion A (responsibility
> separation, manual judgment).

### P10 Remaining Cleanup and Status Corrections

Source: [06](06-cleanup-inventory.md) §Implementation Points

- [ ] **S2 Delete purely-placeholder dead code** (1 level-3 task)
- [ ] **S3 Empty design documents** (1 level-3 task)
- [ ] **S4 RFC status corrections** (1 level-3 task)
- [ ] **S5 Reduce visibility of `pub` items** (1 level-3 task)
- [ ] **S6 opcode dead-path cleanup** (1 level-3 task)

| Level 2                                | Level 3                                                                                                                                                                                                                                                                                                                                                                                               | Acceptance                                                                                                                                        |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| S2 Delete purely-placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladder / old-syntax probe / `pipeline/tests` (14 lines) / `undefined/` empty directory; **E2: the two `_ => todo!()` at `lib.rs:568` / `:582` change to meaningful degraded output**                                                                                                                                                        | `cargo test` passes; `clippy -D warnings` no new warnings                                                                                         |
| S3 Empty design documents              | After deleting C1+C2, rewrite to point at `src/frontend/module/` (~2,670 lines); C3 rewrite the boundary table; **simultaneously clean up `config.js:259-268`**                                                                                                                                                                                                                                       | No 404s on the doc site; `check-docs-truth.py` passes                                                                                             |
| S4 RFC status corrections              | C4 RFC-018 moved back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 add an "implementation status" column to `TRACKING.md` (D36)                                                                                                                                                                                                                                                 | `check_tracking.py` exit code 0 (**do not hand-edit `TRACKING.md`**)                                                                              |
| S5 Reduce visibility of `pub` items    | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be preserved**)                                                                                                                                                                                                                                                                                                                    | Compiler can point out real dead code                                                                                                             |
| S6 opcode dead-path cleanup            | B8 `TailCall` and B9 `Switch` (D32/D34: delete; opcode-value reclamation already landed with the version bump in 7e.1); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes via the `op` field); **F7+F8 opcode generation-time gate** (extend `tools/code-tables` opcode extractor by ~30 lines + consolidate 5 fact-expression sites to a single source + full-opcode `size()` cross-comparison test) | 5 opcode fact expressions consolidated into 1 authoritative source; `size()` comment-and-actual-encoding mismatch (`bytecode.rs:2181-2182`) fixed |

## Implementation Notes: Dependencies and Parallelism

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

> **P3 stop-bleeding channel (2026-10-05 ruling)**: P3's prerequisites are narrowed to **2.3.2 +
> 2.4.1 + 2.4.3** (multifile corpus layer + two red criteria). The rest of P2 (2.1 IR verifier, 2.2
> snapshots, 2.3.1 / 2.3.3 differential and performance baseline) can proceed in parallel with P3.
> Stopping the bleed on the correctness vulnerability is not blocked by criterion-infrastructure
> construction; but **P4 must still wait for all of P2** — the unified Driver needs all three layers
> of criteria in place.

| Constraint         | Reason                                                                                                                                                                   |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P0 → P1            | The three prohibitions constrain every action of P1 (1.2.x is itself adding `mod` declarations to `mod.rs`)                                                              |
| P1 → P2            | Resurrecting tests changes the test count and corpus baseline                                                                                                            |
| P2 (narrowed) → P3 | The vulnerability criteria must **be written as failing first** (2.4.1 / 2.4.3) + the multifile corpus layer (2.3.2) must exist for the red tests to have a place to run |
| P3 → P4            | Fix bugs first, then refactor; the reverse order lets the bugs become "established behavior" baked into the stage table                                                  |
| P4 → P5            | The `include!` refactor and the checker split touch the same file                                                                                                        |
| P5 → P6            | Consecutive changes to the same file must be split, otherwise regressions cannot be bisected                                                                             |
| P6 → P7 / P8       | If the type representation is not unified first, the new IR will grow into a third representation                                                                        |
| P7/P8 → P9         | 9.5's snapshot baseline and 9.3's `pub(crate)` leakage count only take their final form after P7/P8                                                                      |

### Parallelizable Groups

| Group                                      | Members                                                                            | Basis                                                                                                                                                                                                                                        |
| ------------------------------------------ | ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A (the only one authorized by the RFC)** | P7 ∥ P8                                                                            | Disjoint file sets: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*`                                                                                                                                                |
| B                                          | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                      | Each adds `mod` independently, revert file-by-file                                                                                                                                                                                           |
| C                                          | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                    | The three layers of criteria are independent of one another                                                                                                                                                                                  |
| D                                          | 4.4 (proof layer wrap-up) ∥ the entire P6 line                                     | `02` explicitly states S5 "should run independently of S1–S4"                                                                                                                                                                                |
| E                                          | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                              | All independent after 4.1 is complete                                                                                                                                                                                                        |
| F                                          | 9.3 ∥ 9.9 ∥ 9.10 (the three scripts introduced in P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                                                                                                                                                                   |
| G                                          | S3 ∥ S4 ∥ S5                                                                       | Disjoint file sets (verified 2026-10-05: S3 touches `docs/src/dev/design/check/` + `config.js`; S4 touches `docs/src/rfc/` + `check_tracking.py` generator; S5 touches two `pub` items in `src/frontend/core/typecheck/`; pairwise disjoint) |

**Not parallelizable (overlapping files)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5
same-batch files); 4.3.2 ∥ 9.2 (already unified into P4).

**Internal-document additional constraints**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial;
7a→7b→7c→7d strictly serial, and 7b's `Phi` arm must precede 7d.

## Key Decisions and Rationale

- **P0 is a separate stage, not merged into P9** — gates are "post-hoc checks"; rules are "pre-hoc
  judgments". P9's scripts cannot prevent "patching where a refactor is needed"; only D0–D4's review
  checklist can.
- **P9 as the sole authoritative list of CI scripts** — the three coexisting lists from original gap
  G3 made close-out impossible. After this table consolidates, 10 scripts each have a single stage
  owner, and report-only and harden-switching timing are explicitly traceable.
- **P5's steps are filled in by this document rather than merged into P4** — P4 is already the
  largest single-point risk; stacking the checker split on top would exceed rollback granularity.
- **5.3.2 takes "zero caller-side changes" as its hard acceptance** — the split must be
  independently revertible.

## Known Limitations and Risks

- **1.3.x (fixes for defects surfaced) has no regression criterion** — only states "fixed defects
  should not be rolled back", missing the recording requirement of "what was fixed". **Execution
  convention**: every 1.3.x fix's commit message must reference the fixed test name and the defect
  description, serving as a post-hoc bisect basis.
- ~~**7b.1 granularity too coarse**~~ **Fixed (2026-10-05)**: split into dedicated commits 7b.1a /
  7b.1b per DoD 6.
- **8.2.1 granularity too coarse** — ~89 rename references; missing one **silently changes
  associativity**.
- ~~**D17 / D20 had no corresponding level-3 task**~~ **Fixed (2026-10-05)**: D17 landed as 7e.1,
  D20 landed as 4.5.1/4.5.2; D32/D34's opcode reclamation merged into 7e.1; the deletion action is
  left for P10 S6.
- ~~**The 2315 / 2341 data-loss defects were originally "independent issues"**~~ **Fixed
  (2026-10-05)**: absorbed as 7e.2 / 7e.3. The exception table (throw/try) is core language
  semantics and must not be left as outstanding implementation debt.
- ~~**E2 (`lib.rs:568/582` `todo!()`) had no stage ownership**~~ **Fixed (2026-10-05)**: merged into
  P10 S2.
- ~~**F7 (opcode gate extractor) had ambiguous ownership**~~ **Fixed (2026-10-05)**: merged into P10
  S6's generation-time gate task.

## Conflict Log: All Resolved

> **The original "Conflict Log" C1–C8 and the open questions in each document have all been resolved
> in the [RFC-039 Decision Log](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).** This
> document does not leave any pending items. Item-by-item correspondence:

| Original # | Topic                                                                | Ruling                                                                                                                                                                 |
| ---------- | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1         | `precedence_inline.rs` resurrected in P1, deleted in P8              | **Resurrect** (P1's premise is to resurrect tests with real assertions); P8's 8.6 deletes this file along with Pratt                                                   |
| C2         | "Grammar-driven" commitment vs. table lookup                         | **Done in full**: LALRPOP is adopted, see `05` "Syntax: Full Grammar-Driven"                                                                                           |
| C3         | C6 category definition conflicts with S1's classification            | **S1 is assigned to C6** (resurrection does not need equivalence criteria, only regression tests); C6's definition is expanded to "pure deletion or pure resurrection" |
| C4         | `test_release_plan_spans_consumed` whitelist                         | **No whitelist**, the diff set must be empty; if it cannot be empty, the `ReleasePlan` contract is defective → switch to `PlanId` (D20/D41)                            |
| C5         | Internal differences of the two `Aggregation` functions not verified | **P4's 4.2.7 lists it as a required verification item**, line-by-line confirmation during implementation                                                               |
| C6         | wasm playground fix requires P4's `ProgramKind`                      | **The wasm-path vulnerability fix is merged into 4.2.8**; P3 only handles the three native entries                                                                     |
| C7         | `method_def_ordinals` ownership                                      | **Owned by 04** (P7 batch c), see D21                                                                                                                                  |
| C8         | `upvalue_count` correction and `.42` version number                  | **Fix + `VERSION` bumped from 4 to 5** (the format header already has `MAGIC` + `VERSION` fields, `codegen/bytecode.rs:14-16`), see D17                                |

## See Also

- [RFC-039 main text](../../rfc/accepted/039-compiler-architecture.md) — the sole authority for the
  stage sequence, DoD, and global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 criterion category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — detailed design of P0 and the D0–D4
  decision procedure
- [01-routing.md](01-routing.md) — target directory structure (post-completion form)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — design basis for each stage
