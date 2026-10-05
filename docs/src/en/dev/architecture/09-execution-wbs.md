# Multi-Level Construction Task Table (WBS)

> **Companion design document**. This document is a companion to
> [RFC-039: Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md), expanding
> the RFC's phase sequence into **independently committable, independently verifiable** three-level
> tasks.
>
> Phase order and acceptance gates are governed by RFC-039's main text as the sole authority; this
> document only breaks work down to executable granularity and registers unresolved conflicts.

## Positioning and Scope

| Level       | Count       | Meaning                                                                       |
| ----------- | ----------- | ----------------------------------------------------------------------------- |
| **Level 1** | 11 (P0–P10) | Phases, corresponding one-to-one with RFC-039's phase sequence                |
| **Level 2** | 45          | Task groups, drawn from the "Implementation Points" sections of each document |
| **Level 3** | 122         | Independently committable actions                                             |

**This document covers**: task breakdown, dependencies, parallel grouping, binding to acceptance
criteria.

**This document does not cover**: the design rationale of tasks (see `02`–`08`), or the
justification for phase ordering (see RFC-039).

## Current State: Gaps Discovered During Breakdown

When consolidating the "Implementation Points" from the 8 documents into this table, **5 hard gaps
in the documentation system itself** were discovered. All have been converged:

| #   | Gap                                                                                                                                                                                                                                                                                                                                                  | Convergence Result                                                                                                                                                        |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | **P5 has no construction steps at all**. RFC defines P5 (checker split) and acceptance (C1 zero-diff), but none of the 7 documents describe how to perform the split; `01-routing.md`'s directory tree mentions `annotations.rs` once, the only mention in the entire batch                                                                          | P5 steps are filled in by this document (see below)                                                                                                                       |
| G2  | **`include!` refactoring: three attributions, zero landing sites**. RFC lists it in P4; `02`'s "see also" attributes it to itself; `06`'s S7 also lists it. But `02`'s change list (6 additions + 20 modifications) **does not include `semantic_tokens.rs`**                                                                                        | Construction steps attributed to this document §P5 5.1 (after P4 completes); RFC and 06's references have been synchronized to point here                                 |
| G3  | **Three coexisting CI script lists**. `RFC` lists 4 `scripts/ci/*.py`; `01` lists 4 scripts + 1 `cargo test`; `07` lists 4 different checks (all differently named); `04` additionally needs `check-synth-boundary.py`. After deduplication, **at least 9 new scripts with no unified list**; `check-obligations.py` is attributed to both P4 and P9 | Unified list in this document §P9 (10 scripts, each attributed to one phase); all other documents refer to this table                                                     |
| G4  | **`precedence_inline.rs` life-and-death conflict**. `06` counts it among "1005 lines to revive", but the same document's B6 also says it and the `Precedence` enum "must be deleted together", and `05` advocates deletion                                                                                                                           | **Revive in P1, delete in P8 along with Pratt** (decision registered as C1) — first restore CI coverage, then deletion happens together with the dead ladder in 8.6 of P8 |
| G5  | **L2 goal statement conflict**. RFC and this directory's index say "grammar-driven syntax", but `05` self-assesses "**did not adopt true grammar-driven (LALRPOP), only declarative table lookup**"                                                                                                                                                  | Converged to **LALRPOP fully grammar-driven** (05 §Syntax: Full Grammar-Driven); G7/G8 acceptance gates executed per that commitment                                      |

## Target Design: Phase Sequence

```
P0  Repository Maintenance Mechanism & Code Placement Rules  08        ← New, establish rules first
P1  Revive Orphan Tests & Fix Defects                          06 §S1
P2  Establish Equivalence Oracle Baseline                      07 all
P3  Fix Correctness Vulnerabilities (Minimal Solution)         02 §S3 - vulnerability half
P4  Phase Contract & Unified Driver                            02
P5  In-File Checker Split                                      02 + filled in by this document
P6  Type Representation Unification                            03
P7  Intermediate Representation SSA-ification                  04      } parallelizable
P8  Frontend Paradigm Change                                   05      }
P9  Anti-Rebound Gates (incl. Unified Script List)             01 §Anti-Rebound + 08 §Machine-Checkable Rules
P10 Remaining Cleanup & Status Corrections                    06 §S2/S3/S4/S5/S6
```

**Reason for adding P0**: The three prohibitions (no fabrication / no endless padding / patches
instead of refactoring) constrain every action in P1–P10. Rules must be established before work
begins; otherwise when P1 revives tests, additions to `mod` declarations in `tests/mod.rs` are
already happening — which is exactly what caused the G4 conflict.

## Detailed Design: Three-Level Task Table

### P0 Repository Maintenance Mechanism & Code Placement Rules

Source: [08-maintenance-mechanism.md](08-maintenance-mechanism.md)

| L2                       | L3                                                                                                                                                             | Prerequisite | Acceptance                                                                                                                                                                         |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1 Rules formalized     | 0.1.1 Write D0–D4 and the three prohibitions into `CONTRIBUTING.md`; responsibility categories reference the directory responsibility table in `01-routing.md` | —            | Pass manual review                                                                                                                                                                 |
| 0.2 Gate implementation  | 0.2.1 `check-concepts.py` (Prohibition 1 A/B/C/D)                                                                                                              | 0.1.1        | **Must report 3 sets of operator enums and 2 sets of parallel type representations (including `ir::Type` aliases) without code changes**; report-only at this phase, hardens in P9 |
|                          | 0.2.2 `check-fanout.py` (Prohibition 3 A/B/C)                                                                                                                  | 0.1.1        | **Deliberately adding a 6th entry-style wiring must fail**; report-only at this phase, hardens in P9                                                                               |
|                          | 0.2.3 `check-boundary.py` (Prohibition 2 criterion C)                                                                                                          | 0.1.1        | **Deliberately adding a reverse L2→L3 `use` must fail**; record `pub(crate)` leak count as initial value                                                                           |
| 0.3 Table gate promotion | 0.3.1 Extend `tools/code-tables` to cover opcode / type tables                                                                                                 | 0.1.1        | Depends on P6's T1; closed independently                                                                                                                                           |

> **No scale baseline task**. The 2026-10-03 decision cancelled all line-count / size gates; scale
> issues are resolved through responsibility separation (see `08` Prohibition 2 criterion A, manual
> judgment). **0.2.1 is the core acceptance for this phase**: if the gate cannot report the known
> parallel representations, the criterion is misdesigned — it's a false gate.

### P1 Revive Orphan Tests & Fix Defects

Source: [06](06-cleanup-inventory.md) §Implementation Points S1 + [05](05-frontend-paradigm.md)
§Test Reconstruction

| L2                                           | L3                                                                                                                                                         | File:Line                  | Prerequisite | Acceptance                                                                         |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------ | ---------------------------------------------------------------------------------- |
| 1.1 Revive `lexer/tests/`                    | 1.1.1 Add `#[cfg(test)] mod tests;`                                                                                                                        | after `lexer/mod.rs:106`   | —            | Test count increases                                                               |
|                                              | 1.1.2 Add `mod lexer_mod;` / `mod symbols;`                                                                                                                | `lexer/tests/mod.rs:15-25` | 1.1.1        | **Without these 159 lines still don't run**                                        |
|                                              | 1.1.3 Delete 7 empty shell files + their `mod` declarations                                                                                                | 7 files in `lexer/tests/`  | 1.1.1        | C6 has no criterion                                                                |
|                                              | 1.1.4 Delete the `pub use ...::*;` re-export block                                                                                                         | `tests/mod.rs:28-38`       | 1.1.3        | This block is precisely what masked the empty shells                               |
| 1.2 Revive 4 locations in active directories | 1.2.1–1.2.4 respectively wire up `pratt/tests/precedence_inline.rs`, `passes/tests/overload_inline.rs`, `emitter/tests/json.rs`, `package/template/tests/` | each parent module         | —            | Test count increases; **1.2.1 conflicts with 8.0.1, see conflict registration C1** |
| 1.3 Fix exposed defects                      | 1.3.1 Four radix scanner overflow paths                                                                                                                    | `literals.rs`              | 1.1.x        | `cargo test` green                                                                 |
|                                              | 1.3.2 `\x`/`\u` illegal escapes + `scan_leading_dot`                                                                                                       | `literals.rs`              | 1.1.x        | Same as above                                                                      |

**Acceptance**: Test count increases; **real defects are expected to be exposed**, reserve time for
fixes. Revert per file, but **fixed defects should not be rolled back**.

### P2 Establish Equivalence Oracle Baseline

Source: [07](07-equivalence-oracle.md) all (that document has no "Implementation Points" section,
reverse-engineered from structure)

| L2                                 | L3                                                                                                                                                        | Prerequisite | Acceptance                                                                           |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------ |
| 2.1 IR static verifier             | 2.1.1 `verify.rs` dual-mode `verify_loose` / `verify_ssa`                                                                                                 | —            | C4                                                                                   |
|                                    | 2.1.2 **`verify_loose` runs green on existing non-SSA IR**                                                                                                | 2.1.1        | **Hard threshold, admission condition for P7 batch a**                               |
|                                    | 2.1.3 Implement 7 invariants (dominance / unique definition / Phi consistency / jump targets / global out-of-bounds / type consistency / inner isolation) | 2.1.1        | C4                                                                                   |
| 2.2 Normalized snapshots           | 2.2.1 Normalization tool (strip Span / rename temporaries / relativize slots / sort predecessors)                                                         | —            | C1/C3/C5                                                                             |
|                                    | 2.2.2 Snapshot check-in + manual review process (`src/middle/core/tests/snapshots/`)                                                                      | 2.2.1        | C1/C3/C5                                                                             |
| 2.3 Corpus diff                    | 2.3.1 Diff framework + 293 corpus baseline                                                                                                                | —            | Diagnostics/exit code/stdout per item                                                |
|                                    | 2.3.2 **Multi-file corpus layer** (new `tests/yaoxiang-multifile/`, with `yaoxiang.toml` project fixtures — decision D48)                                 | —            | **Unconditional must-do (D40), the only executable behavioral oracle source for P4** |
|                                    | 2.3.3 C4 behavioral diff mandatory coverage list (10 semantic categories) + performance baseline (criterion smoke benchmark)                              | —            | C2/C4/C5; performance baseline for P4/P7/P8 comparison                               |
| 2.4 Vulnerability-specific oracles | 2.4.1 `test_multifile_proof_obligation_not_dropped`                                                                                                       | 2.3.2        | **Must first be red**                                                                |
|                                    | 2.4.3 `test_no_silent_pass_on_unproven`                                                                                                                   | —            | Blocks `checker.rs:5179/5318/5448`                                                   |
|                                    | 2.4.4 `test_release_plan_spans_consumed`                                                                                                                  | —            | **No whitelist, difference set must be empty (D41)**                                 |

> Original 2.4.2 (`test_program_stage_coverage`) and 2.4.5 (`test_obligations_drained` skeleton)
> **removed from P2**: the `Program` / `Obligations` types they reference don't exist until P4, and
> cannot compile in P2. The former becomes P4's 4.1.4 (no double registration), the latter's
> `#[ignore]` red skeleton is merged into 4.3.1. | 2.5 Regression gates | 2.5.1–2.5.4 see P9 unified
> list | 2.1–2.3 | See P9 |

### P3 Fix Correctness Vulnerabilities (Minimal Solution)

Source: [02](02-stage-contract.md) §Implementation Points S3 vulnerability-fix half

| L2                              | L3                                                    | File:Line                                | Prerequisite | Acceptance                        |
| ------------------------------- | ----------------------------------------------------- | ---------------------------------------- | ------------ | --------------------------------- |
| 3.1 `proof_calls` consumer      | 3.1.1 `proof_calls` made private + sole getter        | `types.rs:29`                            | 2.4.1        | Vulnerability oracle turns green  |
|                                 | 3.1.2 Complete the three consumption points           | `orchestrator.rs:273` / `:450` / `:1374` | 3.1.1        | C2                                |
| 3.2 Second silent discard point | 3.2.1 `Unproven` empty match arm produces diagnostic  | `checker.rs:1303-1314`                   | 2.4.3        | `test_no_silent_pass_on_unproven` |
| 3.3 Panic to diagnostic         | 3.3.1 `.expect()` → `SMTResult::Unknown` + diagnostic | `predicate.rs:34-36`                     | —            | No panic when Z3 is missing       |

**Acceptance**: Vulnerability oracles turn green, and **deliberately reverting fixes must turn them
red again**. **Risk**: adding E4018 on the multi-file path is a breaking change.

### P4 Phase Contract & Unified Driver

Source: [02](02-stage-contract.md) §Implementation Points S1–S5. **The fixes in this phase and P3
must be in separate commits.**

| L2                             | L3                                                                                         | Prerequisite | Acceptance                                                                                                                                                |
| ------------------------------ | ------------------------------------------------------------------------------------------ | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 Declarative stage table    | 4.1.1 `stage.rs` (`Stage` variants + `Stage::ALL` + `StageScope`)                          | P0           | `test_program_stage_coverage`                                                                                                                             |
|                                | 4.1.2 `program.rs` / `unit.rs`                                                             | 4.1.1        | Same as above                                                                                                                                             |
|                                | 4.1.3 `Pipeline::run` routed through Driver (`pipeline.rs:141-227`)                        | 4.1.2        | **Single-file diagnostic set and exit code identical byte-for-byte**                                                                                      |
|                                | 4.1.4 Stage coverage assertion                                                             | 4.1.3        | C2                                                                                                                                                        |
|                                | 4.1.5 Full corpus zero-diff                                                                | 4.1.4        | Diagnostics/exit code/stdout all zero-diff (293 corpus)                                                                                                   |
| 4.2 Entry consolidation        | 4.2.1 `compile_project` slimmed to Program constructor (`orchestrator.rs:99-237`)          | 4.1.5        | Four entries' diagnostic sets identical after unification                                                                                                 |
|                                | 4.2.2 `check_project` (`:273-399`)                                                         | 4.2.1        | Same as above                                                                                                                                             |
|                                | 4.2.3 `check_source_in_project` (`:450`)                                                   | 4.2.2        | Same as above                                                                                                                                             |
|                                | 4.2.4 `compile_embedded_module` (`:1374`)                                                  | 4.2.3        | Same as above                                                                                                                                             |
|                                | 4.2.5 Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                             | 4.2.1        | `check` consistent inside/outside project                                                                                                                 |
|                                | 4.2.6 Delete LSP manual stage sequence (`lsp/handlers/diagnostics.rs:161-227`)             | 4.2.1        | LSP and CLI diagnostic sets identical                                                                                                                     |
|                                | 4.2.7 `Aggregation` parameter-driven two-way choice (`orchestrator.rs:486-494`)            | 4.2.1        | **Verify line-by-line internal differences between the two functions during implementation** (conflict registration C5 adjudicated as a must-verify item) |
|                                | 4.2.8 wasm routed through `ProgramKind::WasmPlayground` (`wasm/src/lib.rs:30-36,42-51`)    | 4.2.1        | See C6                                                                                                                                                    |
| 4.3 Obligation ledger          | 4.3.1 `obligations.rs` + `assert_drained()` (new; includes `#[ignore]` red skeleton first) | 4.1.3        | `test_obligations_drained` turns green                                                                                                                    |
|                                | 4.3.2 Obligation diagnostic W→E upgrade                                                    | 4.3.1        | Manual review each new E                                                                                                                                  |
| 4.4 Proof layer & wasm wrap-up | 4.4.1 `layers/README.md` layer order changed to actual order                               | —            | **Proceeds independently of 4.1–4.3**                                                                                                                     |
|                                | 4.4.2 `default_solver()` changed to `&'static` singleton (`proof/smt/backend.rs:67-72`)    | —            | Cache hit rate observable                                                                                                                                 |
|                                | 4.4.3 Add warning to fallback path (`checker.rs:1283-1293`)                                | —            | —                                                                                                                                                         |

### P5 In-File Checker Split — Filled in by this document (original gap G1)

Source: RFC definition + `01-routing.md` directory tree + hard constraint (`collect_used_in_type`'s
`pub(crate)` path must not change)

| L2                          | L3                                                                                                                                                                                                                                                         | Prerequisite | Acceptance                                                      |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | --------------------------------------------------------------- |
| 5.1 `include!` → real `mod` | 5.1.1 Change `checker.rs:5618`'s `include!` to real `mod` declaration                                                                                                                                                                                      | P4 completes | C1 zero-diff; `semantic_tokens.rs` gains module identity        |
|                             | 5.1.2 Add `use super::*` or item-by-item imports (`include!` era implicitly inherited scope)                                                                                                                                                               | 5.1.1        | `cargo build` green                                             |
| 5.2 Split out `refinement`  | 5.2.1 Migrate refinement block (current `checker.rs` 3862–5452 + 5469–5617, including `ReturnRefinement` / `RefinedWalkCtx`)                                                                                                                               | 5.1.1        | C1 zero-diff                                                    |
|                             | 5.2.2 `collect_refined_binding_checks` signature change: `&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`                                                                                                                       | 5.2.1        | C1; only one call site at `1235`                                |
| 5.3 Split out `annotations` | 5.3.1 Migrate annotation validation (current `3441-3661`, including `is_predicate_head`)                                                                                                                                                                   | 5.2.2        | C1 zero-diff                                                    |
|                             | 5.3.2 **`collect_used_in_type` (`3607`, `pub(crate)`) stays `pub(crate)` and under the `checker` module path** → add `pub(crate) use annotations::*;` at the top of `checker.rs`, so that the import in `inference/statements.rs:16` **remains unchanged** | 5.3.1        | **Zero call-site changes** is the hard acceptance for this step |

**Hard constraint**: `collect_used_in_type` is used cross-module by `inference/statements.rs:16`.
The approach is `pub(crate) use` re-export, making the split a **pure relocation**.

**Only split confirmed large blocks.** Don't split modules of only 200–400 lines — increases
navigation cost, reduces cohesion, yields zero decoupling benefit. The remaining submodules
(signatures / type_defs / imports) wait until files grow back to 1,500 lines.

### P6 Type Representation Unification

Source: [03](03-type-unification.md) §Implementation Points stages 0–6, criterion C3. **Stages 0→6
strictly serial.**

| L2                                         | L3                                                                                                                                                                                                      | File:Line                                                                 | Acceptance                                                                                                             |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 6.0 Gate first                             | 6.0.1 New `tools/type-tables`, implement T1/T2/T3, **T1 reports only without failing**                                                                                                                  | new crate                                                                 | T1 accurately lists 13 forward zero-construction variants **+ reverse bridge reconstruction points listed separately** |
|                                            | 6.0.2 Take full baseline on unchanged code                                                                                                                                                              | —                                                                         | No code changes                                                                                                        |
| 6.1 Name normalization                     | 6.1.1 Count occurrences of non-canonical type names in corpus                                                                                                                                           | `tests/yaoxiang/`                                                         | Determines whether synonyms can be deleted directly                                                                    |
|                                            | 6.1.2 `from_builtin_name` cut synonyms → lexer alias table                                                                                                                                              | `mono.rs:618-643`                                                         | C3                                                                                                                     |
|                                            | 6.1.3 Verify which parameter list `classify_generic_params` consumes                                                                                                                                    | `types.rs:162-165`                                                        | **Must verify line-by-line, cannot assume equivalence**                                                                |
| 6.2 Bytecode type unification              | 6.2.1 Three locations changed to `MonoType`                                                                                                                                                             | `bytecode.rs:812/814/854`, `image.rs:44`                                  | C3 + `dump_bytecode` targeted comparison                                                                               |
|                                            | 6.2.2 Delete `From<MonoType> for IrType`                                                                                                                                                                | `bytecode.rs:2352-2390`                                                   | Same as above                                                                                                          |
|                                            | 6.2.3 `type_table` element type changed to `MonoType`                                                                                                                                                   | `image.rs:44`                                                             | Same as above                                                                                                          |
| 6.3 Dead variant deletion                  | 6.3.1 Delete 11 variants + clear match arms + fallback                                                                                                                                                  | `ast.rs`, `mono.rs:699-708`, `types.rs:836`                               | C3 + `cargo build` green                                                                                               |
|                                            | 6.3.2 **Must be in the same batch as 6.3.1** rewrite reverse bridge                                                                                                                                     | `passes/mono/function.rs:507-529` (`mono_to_ast_type` rebuilds 7 of them) | Generic substitution targeted comparison, otherwise **silently changes behavior**                                      |
| 6.4 Parser data flow                       | 6.4.1 New `probe.rs`                                                                                                                                                                                    | new file                                                                  | C3                                                                                                                     |
|                                            | 6.4.2 Modify `parser_state.rs:46-58`                                                                                                                                                                    | —                                                                         | Same as above                                                                                                          |
|                                            | 6.4.3 Delete `operator_interfaces::spec()` calls (**2 locations**: `declarations.rs:509`, `ast.rs:930`)                                                                                                 | —                                                                         | Same as above                                                                                                          |
|                                            | 6.4.4 Merge two `name_used_as_type*`                                                                                                                                                                    | `ast.rs:847` + `declarations.rs:42`                                       | Same as above                                                                                                          |
|                                            | 6.4.5 `CONST_PARAM_TYPES` (`ast.rs:912`) wired through `NameKind::Builtin`                                                                                                                              | —                                                                         | Same as above                                                                                                          |
| 6.5 AST dead variants                      | 6.5.1 Delete `Expr::FnDef` (16 references)                                                                                                                                                              | `ast.rs:37-43` + 4 consumption sites + 2 exhaustive arms                  | `tests/integration/` 18 modules green (**split into 2 commits**)                                                       |
|                                            | 6.5.2 Delete `Assign.signature_params` + `NamedParen` migration                                                                                                                                         | `ast.rs:241-249`                                                          | Marked **semantic risk**                                                                                               |
| 6.6 Gate hardens                           | 6.6.1 T1 changed from warning to `panic!`                                                                                                                                                               | `tools/type-tables`                                                       | **Deliberately introducing a zero-construction variant must fail**                                                     |
| 6.7 Directory rename (D1, pure move batch) | 6.7.1 `typecheck/`→`sema/`; `middle/core/`→`middle/ir/`; top-level `parser/ast.rs` move out to `ast/`; bytecode domain merge (two `bytecode.rs` → `bytecode/`) + `opcode.rs` move to `middle/bytecode/` | entire repo                                                               | **C1 snapshot zero-diff**; exclusive commit; out-of-bounds `use` baseline does not regress                             |

**Hard acceptance**: `git diff --stat tests/ src/std/` must be empty.

### P7 Intermediate Representation SSA-ification

Source: [04](04-ssa.md) §Implementation Points batches a–d, criterion C4, **strictly serial within
batch**.

| L2                                  | L3                                                              | Prerequisite                        | Acceptance                                                      |
| ----------------------------------- | --------------------------------------------------------------- | ----------------------------------- | --------------------------------------------------------------- |
| 7a Cut multiple definitions         | 7a.1 Delete 3 statement-level reclaim sites                     | **2.1.2 `verify_loose` runs green** | C4                                                              |
|                                     | 7a.2 6 save/restore sites switch to RAII guard                  | 7a.1                                | C1 snapshot zero-diff                                           |
|                                     | 7a.3 Caliber unification (`1798` vs `2001`)                     | 7a.2                                | C1                                                              |
| 7b SSA form switch                  | 7b.1 `ir.rs` structural changes #1-5                            | 7a.3 + **P6 completes**             | C4; **exclusive single commit, cannot incrementally roll back** |
|                                     | 7b.2 `next_temp_reg` changed to `Operand::Value`                | 7b.1                                | Same as above                                                   |
|                                     | 7b.3 `translator.rs` add `Phi` arm                              | 7b.1                                | C4 + `.42` roundtrip test                                       |
|                                     | 7b.4 Linear scan allocator (within u8/255 slot model)           | 7b.3                                | Same as above (+500~1000 lines)                                 |
| 7c Implicit contracts made explicit | 7c.1 `synth.rs` boundary + boundary check script                | 7b.4                                | C4 + new script enters CI                                       |
|                                     | 7c.2 span consumption count                                     | 7c.1                                | Consumption count **zero triggers**                             |
|                                     | 7c.3 `method_def_ordinals` read-only                            | 7c.1                                | Attribution doubtful (C7)                                       |
| 7d Do last                          | 7d.1 `generate_call_expr_ir` split into `CallArgs` + 6 `emit_*` | 7c.x                                | C1 snapshot zero-diff                                           |
|                                     | 7d.2 "Fill 0" fallback changed to return diagnostic             | 7d.1                                | C4                                                              |

**Expectation management**: net line count **increases by 900–1600 lines**. If "shorter code" is the
success criterion at project launch, this phase will be judged a failure.

### P8 Frontend Paradigm Change

Source: [05](05-frontend-paradigm.md) §Implementation Points stages 0–4, criterion C5.

| L2                                    | L3                                                                                                  | Prerequisite                     | Acceptance                                                                                               |
| ------------------------------------- | --------------------------------------------------------------------------------------------------- | -------------------------------- | -------------------------------------------------------------------------------------------------------- |
| 8.0 Dead ladder cleanup (C6)          | 8.0.1 Delete `Precedence` enum + `PrecedenceContext` (96 lines)                                     | **Conflicts with 1.2.1, see C1** | C6 has no criterion                                                                                      |
|                                       | 8.0.2 Bare magic numbers named (`(6,7)` / `(11,1)` / `12`)                                          | —                                | Same as above                                                                                            |
| 8.2 Lexical convergence               | 8.2.1 Four radix scanners merge + three escape merges + multi-line string merge                     | —                                | C5; `literals.rs` reduced by ~500 lines                                                                  |
|                                       | 8.2.2 f-string nested compilation elimination                                                       | 8.2.1                            | **span necessarily changes**, baseline must be built first                                               |
| 8.3 Build LALRPOP grammar             | 8.3.1 Grammar file `grammar/yaoxiang.lalrpop`                                                       | 8.2.x                            | `cargo build` passes; produces AST for 293 corpus (**no comparison**)                                    |
|                                       | 8.3.2 Action code `grammar/actions.rs` (one function per 22 `Expr` variant)                         | 8.3.1                            | Same as above                                                                                            |
|                                       | 8.3.3 Error productions (preserve `Expr::Error` / `StmtKind::Error` placeholder semantics)          | 8.3.2                            | Same as above                                                                                            |
| 8.4 **Dual-parser diff**              | 8.4.1 Both parsers run on 293 corpus + `src/std/tests`, AST normalized then **compared bit-by-bit** | 8.3.3                            | **Valid program ASTs fully equivalent — this is the equivalence proof for the entire grammar migration** |
| 8.5 Cut over                          | 8.5.1 `parse()` changed to call LALRPOP, Pratt kept as `parse_legacy()`                             | 8.4.1                            | Full corpus behavior equivalent + diagnostic code+span identical per item (**no relaxation**, no C5′)    |
| 8.6 Delete Pratt                      | 8.6.1 Delete `nud.rs`(1326) + `led.rs`(451) + two BP ladders + ~89 references                       | 8.5.1                            | Corpus fully green; `git grep BP_` zero hits                                                             |
|                                       | 8.6.2 Delete `is_old_function_syntax` (36 lines, naturally obsolete)                                | 8.6.1                            | Same as above                                                                                            |
| 8.7 `parse_assign_after_target` split | 8.7.1 Split 8 responsibility sections + extract `skip_balanced_parens`                              | 8.6.x                            | This function only dispatches                                                                            |
| 8.8 Test completion                   | 8.8.1 New associativity cases (**red first then green**)                                            | 8.2.x                            | C5                                                                                                       |
|                                       | 8.8.2 Supplement four literal error paths                                                           | —                                | Cover untested paths                                                                                     |
|                                       | 8.8.3 Supplement `pratt/tests/mod.rs` wiring self-check assertion                                   | —                                | Prevent orphan recurrence (**`pratt/tests/` no longer exists after 8.6, changed to `parser/tests/`**)    |

**Risk**: 8.4 dual-parser diff is **an unavoidable equivalence checkpoint**. If it produces
non-equivalence, it must be resolved before 8.5 cutover, not covered up by C5′ after cutover. If
8.6's ~89 BP reference migrations are missed, **associativity silently changes** (no error, only
parsing result changes).

> **8.3–8.6 depend on `08`'s D0/D2**: if grammar files are placed outside `parser/`, or action code
> reverse-references `sema`, it will be blocked at `check-boundary.py`.

### P9 Anti-Rebound Gates (Unified Script List — Filled in by this document for original gap G3)

Source: [01](01-routing.md) §Anti-Rebound Mechanism + [08](08-maintenance-mechanism.md)
§Machine-Checkable Rules

**This is the sole authoritative list of all CI scripts** (10 scripts). Scripts join CI at the
"introduction phase"; those marked report-only harden uniformly in P9 (from "report" to "fail") —
existing violations (parallel representations, disambiguation aliases, etc.) can only be cleared
after P6/P8, hardening early would keep CI red long-term and have the gate bypassed manually, so
when P9 hardens, the gates should turn exactly green. P2's 2.5, P4's 4.3.2, P7's 7c.1 all reference
this table and don't create separate lists.

| #    | Script                    | What it Checks                                                                                                             | Intro Phase                                           | Acceptance                                             |
| ---- | ------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------ |
| 9.1  | `check-stage-contract.py` | `Program::stages()` × `Stage::ALL` bidirectional assertion                                                                 | P4                                                    | Deliberately missing one stage must fail               |
| 9.2  | `check-obligations.py`    | Field appears ≥2 times (definition + write) but no third-file read                                                         | P4 (**sole attribution, no longer duplicated in P9**) | Deliberately leaving one unconsumed field must fail    |
| 9.3  | `check-boundary.py`       | Forbid `include!`; `pub(crate)` cross-layer leaks only decrease; forbid L2→L3 (**sole attribution**, defined by `08`'s P0) | **P0**                                                | Deliberately adding `include!` must fail               |
| 9.4  | `check-test-wiring.py`    | `tests/` directory exists but parent module lacks `mod tests;` → fail                                                      | P1                                                    | Deliberately creating orphan directory must fail       |
| 9.5  | `check-snapshot-drift.sh` | Snapshot has diff but commit message lacks `snapshot-update` marker                                                        | P2                                                    | Same as above                                          |
| 9.6  | `check-ir-verifier.sh`    | Full corpus `verify_loose` non-empty                                                                                       | P2                                                    | Same as above                                          |
| 9.7  | `check-corpus-parity.py`  | Corpus diff non-empty (diagnostic code+span compared per item, **no grouping relaxation**)                                 | P2                                                    | Same as above                                          |
| 9.8  | `check-synth-boundary.py` | `ast::Expr` construction appears outside `synth.rs`                                                                        | P7                                                    | Same as above                                          |
| 9.9  | `check-concepts.py`       | Prohibition 1 A/B/C/D (parallel representations, disambiguation aliases, synonym tables)                                   | **P0** (report-only, hardens in P9)                   | Must report 3 sets of operator enums on unchanged code |
| 9.10 | `check-fanout.py`         | Prohibition 3 A/B/C                                                                                                        | **P0**                                                | Deliberately adding a 6th entry-style wiring must fail |

> **No line-count ratchet script** (`check-file-size.py` + `baseline.toml` cancelled on 2026-10-03).
> Scale issues are resolved by `08` Prohibition 2 criterion A (responsibility separation, manual
> judgment).

### P10 Remaining Cleanup & Status Corrections

Source: [06](06-cleanup-inventory.md) §Implementation Points

| L2                                   | L3                                                                                                                                                        | Acceptance                                                            |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| S2 Delete pure placeholder dead code | `instance.rs:416-830` (415 lines) / dispatch chain / dead ladder / old syntax probe / `pipeline/tests` (14 lines) / `undefined/` empty directory          | `cargo test` passes; `clippy -D warnings` no new warnings             |
| S3 Empty-head design docs            | Rewrite C1+C2 after deletion to point to `src/frontend/module/` (~2,670 lines); rewrite C3 boundary table; **synchronously clean up `config.js:259-268`** | Doc site has no 404; `check-docs-truth.py` passes                     |
| S4 RFC status correction             | C4 RFC-018 moved back to `draft/` (D46); C5 RFC-028 add `impl_status: 'not-started'`; F3 `TRACKING.md` add "Implementation Status" column (D36)           | `check_tracking.py` exit code 0 (**don't manually edit TRACKING.md**) |
| S5 `pub` items lower visibility      | B2 `TypeSystem` / B4 `check_type_equivalence` (**`is_subtype` must be preserved**)                                                                        | Compiler can point to true dead code                                  |
| S6 opcode dead path cleanup          | B8 `TailCall` and B9 `Switch` (D32/D34: delete); B10 `UnaryOp::Not` (D33: `opcode()` distinguishes `op` field); F8 generation-time gate                   | Landing together with `.42` version bump (D17, `VERSION` 4→5)         |

## Implementation Points: Dependencies & Parallelism

### Mandatory Serial Main Chain

`P0 → P1 → P2 → P3 → P4 → P5 → P6 → (P7 ∥ P8) → P9 → P10`

| Constraint   | Reason                                                                                                        |
| ------------ | ------------------------------------------------------------------------------------------------------------- |
| P0 → P1      | Three prohibitions constrain every action in P1 (1.2.x is exactly adding `mod` declarations to `mod.rs`)      |
| P1 → P2      | Reviving tests changes test count and corpus baseline                                                         |
| P2 → P3      | Vulnerability oracles must **be written red first**                                                           |
| P3 → P4      | Fix bugs before refactoring; reversing order lets bugs be固化d into "established behavior" by the stage table |
| P4 → P5      | `include!` refactoring and checker split touch the same file                                                  |
| P5 → P6      | Consecutive changes to the same file must be separated, otherwise regressions can't be bisected               |
| P6 → P7 / P8 | Without type representation converged first, new IR grows into a third representation                         |
| P7/P8 → P9   | 9.5's snapshot baseline and 9.3's `pub(crate)` leak count take final form only after P7/P8                    |

### Parallelizable Groups

| Group                       | Members                                                                        | Basis                                                                                         |
| --------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| **A (sole RFC-authorized)** | P7 ∥ P8                                                                        | Disjoint file sets: `ir.rs`/`ir_gen.rs`/`bytecode.rs`/`translator.rs` vs `lexer/*`/`parser/*` |
| B                           | 1.2.1 ∥ 1.2.2 ∥ 1.2.3 ∥ 1.2.4                                                  | Each independently adds `mod`, revert per file                                                |
| C                           | 2.1 ∥ 2.2 ∥ 2.3.1 ∥ 2.3.2 ∥ 2.4                                                | Three oracle layers independent of each other                                                 |
| D                           | 4.4 (proof layer wrap-up) ∥ entire P6 line                                     | `02` explicitly states S5 "should proceed independently of S1-S4"                             |
| E                           | 4.2.5 ∥ 4.2.6 ∥ 4.2.8                                                          | All after 4.1 completes, mutually independent                                                 |
| F                           | 9.3 ∥ 9.9 ∥ 9.10 (three scripts introduced in P0 can be developed in parallel) | 9.5's snapshot baseline must be taken last                                                    |
| G                           | S3 ∥ S4 ∥ S5                                                                   | Disjoint file sets                                                                            |

**Not parallelizable (file overlap)**: P1(1.1.x) ∥ P8(8.0.x); 1.2.1 ∥ 8.3.2; 6.0 ∥ S2 (B6/E5 same
batch files); 4.3.2 ∥ 9.2 (unified to P4).

**Internal serial constraints**: 6.0→6.1→6.2→6.3→6.4→6.5→6.6 strictly serial; 7a→7b→7c→7d strictly
serial, and 7b's `Phi` arm must precede 7d.

## Key Decisions and Rationale

- **P0 as a separate phase rather than merged into P9** — gates are "post-hoc checks", rules are
  "pre-judgment". P9's scripts cannot prevent "patches instead of refactoring"; only D0–D4's review
  checklist can.
- **P9 as the sole authoritative list of CI scripts** — the three coexisting lists from original gap
  G3 made closure impossible. After this table's merge, 10 scripts are each attributed to one phase,
  with report-only and hardening timing explicitly queryable.
- **P5's steps filled in by this document rather than merged into P4** — P4 is already the largest
  single-point risk; stacking checker split on top exceeds revertable granularity.
- **5.3.2 with "zero call-site changes" as hard acceptance** — the split must be independently
  revertable.

## Known Limitations and Risks

- **1.3.x (fix exposed defects) has no regression criterion** — only says "fixed defects should not
  be rolled back", lacks a recording requirement for "what was fixed".
- **7b.1 granularity too coarse** — `ir.rs` structural changes #1-5 affect the whole repo's
  compilation; `04` itself marks "the only part that cannot be incrementally rolled back", naturally
  not one commit's workload.
- **8.2.1 granularity too coarse** — ~89 rename references; missing one **silently changes
  associativity**.

## Conflict Registration: All Adjudicated

> **The original "Conflict Registration" C1–C8 and the open questions from each document have all
> been converged into the [RFC-039 Decision Registry](../../rfc/draft/039-compiler-architecture.md)
> (D1–D50).** This document leaves no pending items. Item-by-item correspondence:

| Original ID | Topic                                                        | Adjudication                                                                                                                                       |
| ----------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1          | `precedence_inline.rs` revive in P1, delete in P8            | **Revive** (P1's premise is reviving tests with real assertions), P8's 8.6 deletes the file together with Pratt                                    |
| C2          | "Grammar-driven" commitment vs table lookup                  | **Fully done**: adopting LALRPOP, see `05` "Syntax: Full Grammar-Driven"                                                                           |
| C3          | C6 category definition conflicts with S1 classification      | **S1 classified under C6** (revival needs no equivalence oracle, only regression tests); C6 definition expanded to "pure deletion or pure revival" |
| C4          | `test_release_plan_spans_consumed` whitelist                 | **No whitelist**, difference set must be empty; if not empty, `ReleasePlan` contract is defective → change `PlanId` (D20/D41)                      |
| C5          | `Aggregation` two functions' internal differences unverified | **P4's 4.2.7 lists it as a must-verify item**, verify line-by-line during implementation                                                           |
| C6          | wasm playground fix needs P4's `ProgramKind`                 | **wasm path vulnerability fix merged into 4.2.8**; P3 only handles three native entries                                                            |
| C7          | `method_def_ordinals` attribution                            | **Attributed to 04** (P7 batch c), see D21                                                                                                         |
| C8          | `upvalue_count` correction and `.42` version number          | **Fix + `VERSION` from 4 to 5** (format header already has `MAGIC` + `VERSION` field, `codegen/bytecode.rs:14-16`), see D17                        |

## See Also

- [RFC-039 Main Text](../../rfc/draft/039-compiler-architecture.md) — sole authority for phase
  sequence, DoD, and global acceptance gates G1–G10
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 oracle category definitions
- [08-maintenance-mechanism.md](08-maintenance-mechanism.md) — P0 detailed design and D0–D4 decision
  procedure
- [01-routing.md](01-routing.md) — target directory structure (post-completion form)
- [02](02-stage-contract.md) / [03](03-type-unification.md) / [04](04-ssa.md) /
  [05](05-frontend-paradigm.md) / [06](06-cleanup-inventory.md) — design basis for each phase
