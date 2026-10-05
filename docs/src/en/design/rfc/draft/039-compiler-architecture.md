---
title: 'RFC-039: Compiler Architecture Refactoring (Master Plan)'
author: 'ChenXu233'
created: '2026-10-03'
updated: '2026-10-04'
status: 'Draft'
issue: 'TBD'
---

# RFC-039: Compiler Architecture Refactoring

## Summary

This RFC proposes an architecture refactoring for the YaoXiang compiler (`src/` 160,002 lines / 509
`.rs` files): re-establish boundaries along a **four-layer model** (Orchestration / Frontend /
Intermediate Representation / Execution), establish an **exhaustively enumerable compilation stage
contract** and an **obligations ledger**, and accompany them with a set of **equivalence oracles**
and **anti-regression gates**.

The detailed design is split across nine companion documents in
`docs/src/design/compiler-architecture/`. This RFC is responsible only for: the problem, the root
cause, the four-layer model, the graded acceptance criteria, and the execution stage ordering.
**Reviewing this RFC alone is sufficient to decide whether to greenlight the project; the companion
documents are the construction blueprints after greenlight.**

## Motivation

### I. Boundaries exist but have no enforcement

The directory-based layering (`frontend` / `middle` / `backends`) is clear. But **the boundaries
exist only in human self-discipline, with no mechanism ensuring they are not eroded**. Three direct
consequences follow.

#### 1.1 Compilation stages are hand-wired at five entry points, producing 11 behavioral inconsistencies

| Stage                      | Single-file `run` | Multi-file `run` | `check` | LSP (in-project) | LSP (single-file) |
| -------------------------- | ----------------- | ---------------- | ------- | ---------------- | ----------------- |
| lexing / parsing           | yes               | yes              | yes     | yes              | yes               |
| typecheck                  | yes               | yes              | yes     | yes              | yes               |
| **proof_execution**        | **yes**           | **no**           | **no**  | **no**           | **no**            |
| dead code analysis         | yes               | no               | yes     | no               | no                |
| W1006 shadowing diagnostic | no                | no               | yes     | no               | no                |
| monomorphization           | yes               | no               | no      | no               | no                |
| IR generation / linking    | yes               | yes              | no      | no               | no                |

The five entry points are: `src/frontend/pipeline.rs:141-227` (single file, 5 stages at `149` /
`161` / `173` / `188` / `205`), `src/frontend/compiler.rs:106` (wrapper), the four in
`src/frontend/module/orchestrator.rs` (`compile_project:99` / `check_project:273` /
`check_source_in_project:450` / `compile_embedded_module:1374`), plus `src/lib.rs`'s `run_file:140`
/ `run_project:154`.

The most severe cell produces a **correctness vulnerability**. Constraints on proof functions of the
form `x: Sorted(3)` **silently pass** on three paths: multi-file, `check`, and LSP. The evidence
chain (all auditable):

1. `src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** location in the entire
   repository that constructs a non-empty `proof_calls`, with the semantics of "this constraint
   requires executing a proof function".
2. `src/frontend/core/typecheck/checker.rs` has three isomorphic branches handling `Unproven`:
   `5164` / `5306` / `5420` are `if calls.is_empty()` → push a hard error; `5179` / `5318` / `5448`
   are `ctx.proof_calls.extend(...)` → **produces no diagnostic whatsoever**. The comment at `5165`
   literally reads "Unproven → compile error, no degradation, no silent pass… do not let silent pass
   resurrect".
3. `TypeCheckResult.proof_calls` (`types.rs:29`) is **read in exactly one location in the entire
   repository**: `src/frontend/pipeline.rs:187`.
4. All four entry points in `orchestrator.rs` **do not read this field**.

In other words: an invariant declared by the code itself is broken by the architecture, not the
logic. The existence of `compile_embedded_module` (`orchestrator.rs:1374`) means that **the
refinement obligations of the embedded std module itself also flow through this discarding path**.

**No test can catch it**: the 27 tests in `tests/integration/multifile.rs` have zero hits on
`Sorted` / `proof` / `refin`; RFC-027's unit tests call `check_module` directly and assert
"`proof_calls` is non-empty", **stopping exactly before the pipeline** — they verify "filled in",
while the bug is "consumer never reads".

#### 1.2 Module boundaries can be bypassed without a trace

`src/frontend/core/typecheck/checker.rs:5618` is `include!("checker/semantic_tokens.rs");` — **the
only `include!` in the entire repository**. It splices 1547 lines of text into the `checker` module.
This file **is not declared as any module** (`grep 'mod semantic_tokens'` yields zero hits
repo-wide), its first line is `impl TypeChecker {` directly, it has no `use` header of its own, and
`checker/` contains only this one file.

Consequences: no module identity, visibility isolation breaks, rust-analyzer's jump and symbol
search fail on it, toolchain analysis counts it as part of `checker.rs`. **The real size of the
`checker` module is 5618 + 1547 ≈ 7165 lines**, not the 5618 lines shown by directory statistics.

#### 1.3 Test wiring depends on memory, corruption happens silently

Performing BFS reachability from the two crate roots `src/lib.rs` / `src/main.rs`: **24 files / 2628
lines never participate in compilation**. Among these, 23 files / 1081 lines are real orphans.

| Orphan                                      | Size                 | Content                                                                              |
| ------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------ |
| `frontend/core/lexer/tests/` entire subtree | 13 files / 686 lines | 7 empty shells totaling 19 lines + `mod.rs` 38 lines + **629 lines / 55 real tests** |
| `frontend/pipeline/tests/`                  | 3 files / 14 lines   | all placeholder doc comments                                                         |
| `package/template/tests/`                   | 3 files / 65 lines   | 7 `#[test]`                                                                          |
| `parser/pratt/tests/precedence_inline.rs`   | 95 lines / 6 tests   | hidden in a live directory                                                           |
| `typecheck/passes/tests/overload_inline.rs` | 170 lines / 7 tests  | same                                                                                 |
| `util/diagnostic/emitter/tests/json.rs`     | 46 lines / 3 tests   | same                                                                                 |

`cargo test` cannot discover them: tests that have never run will not fail. CI
(`.github/workflows/ci.yml:140`) has no test count baseline or wiring check either.

**A total of 1005 lines / 78 real tests have never run.** The lexer layer's actually-running test is
only `fstring.rs` (spliced in via `#[path]` attribute).

### II. The design capability is sufficient

It must also be acknowledged: this project **already has** mature enforcement mechanisms, just not
generalized.

The error codes in `src/util/diagnostic/codes/` (137 E codes + 8 W codes = 145) are gated at build
time by `build.rs:19-55` through `tools/code-tables` — parsing the registry, validating uniqueness
and segment positions, comparing line-by-line against RFC-013's markdown code table, **and any
inconsistency directly `panic!`-refuses to compile**. RFC-013 documentation and code therefore stay
consistent.

`src/package/` (76 files / 13,012 lines) is the most complete region: **6,227 lines of tests,
47.9%**; 5 of 6 `tests/` directories are correctly wired (the only exception is `template/tests/`,
see Section A). Note that this positive example is established at the **directory-level wiring**,
not at the file level — of the 76 `.rs` files, only 7 contain inline `#[cfg(test)]` and only 5
contain `mod tests;`. 5 RFCs match code line-by-line, and many "not yet implemented" items become
explicit errors rather than silent TODOs (cmake at `build/mod.rs:172-174`, `RegistryDeferred` at
`error.rs:74-76`). `.yxpkg` has five layers of safety protection, all correctly applied.

**In the same repository, by the same authors, `package/` did test directory wiring right, while
`lexer/tests/` has never been run once. The difference is not capability — it's whether anyone runs
a check.**

### III. Phantom design

The three documents under `docs/src/design/check/` describe a **never-existing** architecture:
`CheckSession` (including the Rust code draft in `incremental-checking.md:31-41`),
`ModuleDependencyGraph`, `ModuleCache`, `HotReloader` all have zero hits; the `traits/` directory
does not exist; the `check_single_module` function does not exist (it only lives in test comments);
even the `command.rs` busy-wait referenced in "known limitations" does not exist (that file has
neither `Instant` nor `recv_timeout`).

Cross-file analysis is actually implemented by `src/frontend/module/` (registry 439 + resolver 195 +
roles 388 + orchestrator 1528, approximately 2670 lines) — the capability is real, but the path is
completely different. **This document writes "a possible future design" as "defects in an
already-implemented system".**

`src/frontend/pipeline/tests/compilation_cache.rs` and `incremental_scheduler.rs` (3 lines each) are
the cleanest physical evidence of "built test skeletons per design, discovered the pipeline had no
corresponding implementation, then shelved them".

Furthermore, `RFC-018` (LLVM AOT, 1037 lines, status accepted) has 0 lines of implementation in code
— all 6 AOT-related hits in `backends/mod.rs` are **comments**.

## Root cause diagnosis

Combining the three categories of symptoms, the root cause is single:

> **This project treats "design" as a documentation convention rather than an executable
> constraint.**

| Layer             | Missing enforcement                                             | Existing but not generalized example                                         |
| ----------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Stage boundaries  | A single decision point for "which stages this compilation ran" | —                                                                            |
| Module boundaries | Forbid `include!`, restrict `pub(crate)` cross-layer leakage    | —                                                                            |
| Cross-references  | Detectability of "field produced but no one consumes"           | `build.rs`'s error code gate                                                 |
| Test wiring       | Declaring a directory must come with a `mod` declaration        | `src/package/`'s directory-level wiring convention (5 of 6 `tests/` correct) |

This RFC's stance is therefore: **transform the three categories of boundaries (stage, module,
cross-reference) from "documentation convention" into "compile-time or test-time assertable
facts"**, and **establish a unique stage decision point at the orchestration layer**.

## Proposal

### Four-layer model

```
┌─────────────────────────────────────────────────────────────┐
│ L1 Orchestration                                            │
│   Decides which stages run, how many times, how failures    │
│   propagate                                                 │
│   Current: pipeline.rs(740) + orchestrator.rs(1528)         │
│         + compiler.rs(267) = 5 entry points, 11             │
│         inconsistencies                                     │
│   Target: single Driver + exhaustively enumerable Stage    │
│           enum + Obligations ledger                         │
│   See: ../compiler-architecture/02-stage-contract.md           │
└───────────────────────────┬─────────────────────────────────┘
                            │ Compilation Unit
┌───────────────────────────▼─────────────────────────────────┐
│ L2 Frontend                                                 │
│   Lexing, parsing, AST construction                         │
│   Current: lexer/(3065) + parser/(6341)                     │
│         Hand-written lexer + hand-written Pratt; adding     │
│         an operator requires changes in 6-7 places inside   │
│         L2 + 17 downstream production files                 │
│   Target: lexer converges to a single implementation +      │
│           LALRPOP grammar-driven parser                     │
│   See: ../compiler-architecture/05-frontend-paradigm.md        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Module (AST)
┌───────────────────────────▼─────────────────────────────────┐
│ L3 Semantic & Intermediate Representation                    │
│   Type checking, static analysis, IR construction           │
│   Current: typecheck/(29466 production) + ir.rs(905) +     │
│         ir_gen.rs(8448)                                     │
│         3 parallel type representations + 3 parallel        │
│         operator enums;                                     │
│         ir_gen single impl 7952 lines / 116 methods;       │
│         no IR-level validator                               │
│   Target: type representation unified; IR satisfies SSA     │
│           construction discipline                            │
│   See: ../compiler-architecture/03-type-unification.md         │
│        ../compiler-architecture/04-ssa.md                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ ModuleIR
┌───────────────────────────▼─────────────────────────────────┐
│ L4 Execution                                                │
│   Bytecode generation, interpreter, runtime, stdlib         │
│   Current: codegen/(3436) + bytecode.rs(2422) +             │
│         executor/(2000)                                     │
│         + runtime/(1800) + std/(7543)                       │
│   Target: opcode single source of truth + generation-time   │
│           gate; execution layer unaware of L1-L3            │
│   See: ../compiler-architecture/06-cleanup-inventory.md        │
└─────────────────────────────────────────────────────────────┘
```

**The four layers are strictly unidirectional**: L1 depends on the interfaces of L2/L3/L4; L2 does
not depend on L3; L3 does not depend on L2 (it only consumes AST data, not calling parser); L4 only
depends on the product format of L3.

The known reverse dependencies and parallel definition list is in the routing table C of
`../compiler-architecture/01-routing.md`.

### Companion design documents

Nine, stored in `docs/src/design/compiler-architecture/`:

| Document                          | Layer            | Topic                                                                                                                                                                                                                                      |
| --------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `01-routing.md`                   | Cross-layer      | **Function routing table**, dependency direction specification, **target directory structure after construction completion**, future extension guidelines                                                                                  |
| `02-stage-contract.md`            | L1               | Stage contract and obligations ledger: 11 inconsistencies, vulnerability evidence chain, `Stage` / `Obligations` / `Driver` / `ProgramKind` (including `WasmPlayground` on the wasm playground side)                                       |
| `03-type-unification.md`          | L2/L3 root cause | Type representation unification: 26 `ast::Type` variants handled one by one, 13 forward zero-construction variants, type table gate                                                                                                        |
| `04-ssa.md`                       | L3               | IR SSA-ization: four categories of defects, SSA form definition, register allocator evaluation, 38-item change list                                                                                                                        |
| `05-frontend-paradigm.md`         | L2               | Lexer convergence to a single implementation / grammar driven by LALRPOP, change surface convergence, dead ladder handling, test rebuilding                                                                                                |
| `06-cleanup-inventory.md`         | Global           | Dead code and phantom design cleanup: reachability method, item-by-item handling, execution order                                                                                                                                          |
| `07-equivalence-oracle.md`        | Cross-layer      | Equivalence oracle: three-layer criteria, C1-C6 grading, gate design                                                                                                                                                                       |
| **`08-maintenance-mechanism.md`** | **Cross-layer**  | **Repository maintenance mechanism**: three prohibitions (no fabrication / no endless padding / patching instead of refactoring), D0–D4 decision procedure, machine-checkable rules, code review checklist, external convention references |
| **`09-execution-wbs.md`**         | **Cross-volume** | **Multi-level execution task table**: 11 first-level / 45 second-level / 122 third-level tasks, dependencies and parallel grouping, **8 conflict registrations**                                                                           |

`01` and `09` are **long-term reference documents** — the former does not become invalid with any
refactoring, the latter is the construction checklist. The three prohibitions produced by `08`
constrain every action of P1–P10.

### Equivalence oracle grading

**This is the safety net for all refactoring and must be established before any code change.**
Detailed design in `07-equivalence-oracle.md`.

Core principle: **criteria are graded by refactoring category, because different categories require
different strengths of equivalence**. Using a single criterion (usually "IR snapshot full equality")
to cover all refactorings is wrong — some refactorings **will necessarily change IR form**.

| Category | Refactoring content                                                   | Criterion type                                            | Strength  |
| -------- | --------------------------------------------------------------------- | --------------------------------------------------------- | --------- |
| **C1**   | Pure relocation (split files, change directories, extract submodules) | IR normalized snapshot **zero-diff**                      | Strongest |
| **C2**   | Orchestration change (staging, unified Driver)                        | Same diagnostic set at each entry + same corpus behavior  | Strong    |
| **C3**   | Type representation convergence                                       | Same diagnostic **code** (message wording may change)     | Strong    |
| **C4**   | IR form change (SSA-ization)                                          | **Behavioral equivalence** + IR structural invariants     | Medium    |
| **C5**   | Frontend paradigm change                                              | AST snapshot + diagnostics + behavior, all three used     | Strong    |
| **C6**   | Pure deletion                                                         | No equivalence needed, only confirmation of no references | —         |

**The distinction between C1 and C4 is key**: C1 requires IR byte-for-byte equality; C4 acknowledges
IR will change, and switches to "same program behavior + IR satisfies invariants". Forcing snapshots
on C4 will induce the team to relax the criterion.

Three-layer criteria:

1. **IR static validator** — dominant use-before-def, jump target existence, type consistency,
   inner-layer isolation. Far stronger than snapshots, covering defects like "IR self-consistent but
   value wrong" that snapshot tests cannot catch. Must **first run green on the existing (non-SSA)
   IR**.
2. **Normalized IR snapshot** — strip `Span`, rename temporaries in order of appearance, relativize
   global slots, sort predecessors. Commit, manually review diff. **Known limitation**:
   normalization erases "which register the Nth argument used", so **alone insufficient to cover
   `arg_regs` semantic reshuffling**.
3. **End-to-end corpus differential** — diagnostic list, exit code, stdout/stderr compared item by
   item.

> **⚠️ Structural limitation of corpus coverage**: there is **not a single `yaoxiang.toml`** under
> the `tests/` directory (verified 0), therefore the **293** `.yx` corpus files in `tests/yaoxiang/`
> **all take the single-file path** (the standalone branch of `check_files_with_diagnostics` →
> `check_single_file` → `Pipeline::run`).
>
> **Consequence**: the third-layer criterion **can only verify changes on the single-file path**.
> For the multi-file side (the four entry points of `orchestrator`) and multi-file behavior of
> types/SSA, **there is currently no corpus coverage whatsoever**. P4 "Unified Driver" is the
> highest-risk point of the entire plan, **the multi-file corpus layer must be unconditionally
> established in P2 (resolution D40)** — it is the sole executable behavioral criterion source for
> P4.

## Implementation Strategy

### Stage sequence

```
P0  Repository maintenance mechanism and code placement protocol 08   ← First set rules: three prohibitions + D0-D4 decision procedure
P1  Revive orphan tests and fix defects             06 §S1            ← Cheapest, biggest payoff
P2  Establish equivalence oracle baseline           07 all
P3  Fix correctness vulnerability (minimal solution) 02 §Fix
P4  Stage contract and unified Driver                02
P5  checker file internal split                      09 §P5 supplement
P6  Type representation unification                  03
P7  IR SSA-ization                                   04     } parallelizable
P8  Frontend paradigm change                         05     }
P9  Anti-regression gates (unified script list)      01 + 08
P10 Other cleanup and status corrections             06 §S2-S6
```

**Core serial constraints (non-swappable):**

| Constraint   | Reason                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1      | The three prohibitions constrain every action of P1 — 1.2.x is precisely adding `mod` declarations to `mod.rs`, which is exactly how the `precedence_inline.rs` life-and-death conflict arose                                                                                                                                                                                                                          |
| P1 → P2      | Reviving tests will change test count and corpus baseline. Establishing baseline first will fail after P1                                                                                                                                                                                                                                                                                                              |
| P2 → P3      | The vulnerability criterion must **be written as red first** to prove the fix effective. **Narrowing (D51)**: P3's prerequisites are only 2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red criteria); the rest of P2 can run in parallel with P3, but P4 must wait for P2's full completion — stopping the bleeding of the correctness vulnerability must not be blocked by criteria infrastructure construction |
| P3 → P4      | Fix the bug first then refactor. Reverse order will let the bug be solidified as "established behavior" by the stage table                                                                                                                                                                                                                                                                                             |
| P4 → P5      | P5 opens with `include!` → real `mod` (09 §P5 5.1), and the subsequent checker split operates on the same file, must proceed consecutively after P4                                                                                                                                                                                                                                                                    |
| P5 → P6      | Consecutive changes to the same file must be separated, otherwise regressions cannot be bisected for localization                                                                                                                                                                                                                                                                                                      |
| P6 → P7 / P8 | Without first converging type representation, the new IR from SSA will grow into a third representation                                                                                                                                                                                                                                                                                                                |
| P7/P8 → P9   | The initial baselines for each gate script (e.g., `pub(crate)` leak count) only take final values after P7/P8                                                                                                                                                                                                                                                                                                          |
| P7 ∥ P8      | File sets are disjoint, parallelizable                                                                                                                                                                                                                                                                                                                                                                                 |

**Why P0 is an independent stage**: P9's CI scripts are **post-hoc checks**, they can only catch
already-written bad code; "should refactor but patched instead" is a **prior judgment**, only the
D0–D4 decision procedure can intercept it. The two cannot replace each other.

### Each stage's key points

| Stage   | Touch scope                                                                                                                                                                                                            | Acceptance                                                                                                                                                                                                                          | Rollback point                                                                                  |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **P0**  | `CONTRIBUTING.md` protocol section + 3 new gate scripts (`check-concepts` / `check-fanout` / `check-boundary`); **no compiler source code change, no line-count gate**                                                 | `check-concepts.py` **must report 3 sets of operator enums, 2 sets of parallel type representations (including `ir::Type` aliases) on unmodified code**; deliberately exceeding limit / adding a 6th entry-style wiring must go red | Pure addition, deleting the script is enough                                                    |
| **P1**  | 5 files add `mod` declarations; **1005 lines / 78 tests revived**                                                                                                                                                      | Test count rises; **expected to expose real defects** (`literals.rs` overflow path, `\x`/`\u` illegal escapes never tested)                                                                                                         | Per-file revert; **fixed defects should not be rolled back**                                    |
| **P2**  | New `verify.rs`, snapshot baseline, corpus differential framework, **multi-file corpus layer**, performance baseline (criterion smoke benchmark), `scripts/ci/check-*`                                                 | `verify_loose` runs green; vulnerability criterion **written as red**                                                                                                                                                               | Criterion code can be removed as a whole                                                        |
| **P3**  | `types.rs` (`proof_calls` private), 3 consumer points in `orchestrator.rs`                                                                                                                                             | Vulnerability criterion **turns green**; **deliberately removing the fix must turn red again**                                                                                                                                      | Pure behavior fix                                                                               |
| **P4**  | New `src/driver/`; rewrite 5 entry points; **4.5: `ReleasePlan` / `overload_resolutions` key `Span` → `PlanId` (D20)**                                                                                                 | Same diagnostic set + same corpus behavior (C2); span-keyed silent failures go to zero                                                                                                                                              | **Highest risk point**; keep old entry points, un-hooked Driver allows rollback                 |
| **P5**  | `include!` → real `mod` (5.1); split `refinement` / `annotations` out of `checker.rs`                                                                                                                                  | **IR snapshot zero-diff** (C1); `pub(crate)` path at `statements.rs:16` unchanged                                                                                                                                                   | Per-file revert                                                                                 |
| **P6**  | `ast.rs`, `types/mono.rs`, `solver.rs`, `ir.rs:3`, `bytecode.rs:2353-2390`; phase wrap-up executes directory renames (D1: `typecheck/`→`sema/`, `middle/core/`→`middle/ir/`)                                           | Same diagnostic **code** (C3); rename batches are pure relocation (C1 zero-diff)                                                                                                                                                    | Phased commits, variant handling and synonym table deletion separated; rename in its own commit |
| **P7**  | `ir.rs`, `ir_gen.rs` (8448 lines), `bytecode.rs`, `translator.rs`; **7e: fix three `.42` data losses (`upvalue_count` / `exception_handlers` / `globals`) + `VERSION` 4→5 (D17, D52)**                                 | **Behavioral equivalence + `verify_ssa` green** (C4). **Not IR snapshot equality**; `.42` roundtrip tests                                                                                                                           | Per-batch revert                                                                                |
| **P8**  | `lexer/*`, `parser/*`                                                                                                                                                                                                  | AST snapshot + diagnostics + behavior (C5)                                                                                                                                                                                          | Per-file revert                                                                                 |
| **P9**  | New `scripts/ci/check-*.py` totaling 10 (the only list is in [09](../../compiler-architecture/09-execution-wbs.md) §P9: 3 with P0, 1 with P1, 3 with P2, 2 with P4, 1 with P7 introduced, P9 close-up unifies to hard) | **Deliberately having obligations with no consumer must go red**; **deliberately creating an orphan test directory must go red**; **deliberately adding `include!` must go red**                                                    | Delete the script                                                                               |
| **P10** | Document disposal, RFC status correction, `pub` items downgraded in visibility, opcode decisions                                                                                                                       | `check_tracking.py` passes; docs site no 404s                                                                                                                                                                                       | Independent PR                                                                                  |

### Definition of Done (DoD) for each stage

A stage counts as done only when all of the following are simultaneously satisfied:

1. All corresponding "implementation strategy" steps in the assigned document are committed
2. The criteria of that category (C1-C6) in `07-equivalence-oracle.md` are **green in CI**
3. `cargo test` all green, and **test count is not lower than at the start of that stage** (prevent
   "delete tests for green light")
4. `cargo clippy --all --all-features -- -D warnings` clean
5. `python scripts/rfc/check_tracking.py` exit code 0
6. The stage has an independent revert unit — **if a stage cannot be reverted independently, it is
   too large and needs to be split further**
7. Stages that touch the compilation pipeline or execution path (P4 / P6 / P7 / P8): 07's
   performance baseline comparison has no unexplained >10% regression

### Global acceptance gates

After all stages are complete, the following must hold:

| #   | Condition                                            | Verification                                                                       |
| --- | ---------------------------------------------------- | ---------------------------------------------------------------------------------- |
| G1  | The decision point for compilation stages is unique  | Answering "which stages ran, which didn't, why" only requires looking at one place |
| G2  | No "produced but no one consumes" fields             | `check-obligations.py` green                                                       |
| G3  | `include!` count is 0                                | `grep -rn 'include!' src/`                                                         |
| G4  | No orphan tests                                      | `check-test-wiring.py` green                                                       |
| G5  | No reverse dependencies                              | `check-module-boundary.py` green                                                   |
| G6  | Type representation unique                           | Parallel representations converged; hand-written synonym table gone                |
| G7  | parser contains no type/predicate hardcoding         | Literals like `"Terminates"` replaced by data flow                                 |
| G8  | Adding a binary operator's change surface ≤ 3 places | Manual review + routing table update                                               |
| G9  | `layers/README.md` describes actual layer order      | Documentation consistent with `check_module_impl`                                  |
| G10 | `TRACKING.md` has an "Implementation Status" column  | Implementation status of 52 RFCs queryable                                         |

## Trade-offs

### Advantages

- **Provides a landing point for all subsequent changes**. "Adding an operator requires changing a
  dozen places, with no one noticing" cannot be improved by memory, only by routing tables + gates.
- **Single root cause**. Three categories of symptoms (stage inconsistency, `include!`, test wiring
  break) all stem from "boundaries are conventions not constraints", solvable in one shot with the
  same category of mechanism.
- **Non-blocking**. This RFC has zero code changes. The four-layer model is descriptive, and after
  adoption, only parts of it may still be executed.
- **Criteria first**. P2 is established before any code change, and the graded design avoids the
  common failure of "snapshot too strict → induced relaxation".

### Disadvantages and risks

- **The serial chain is long**. P1 → P2 → P3 → P4 → P5 → P6 → P7/P8 is nearly fully serial, later
  stages must wait for earlier ones.
- **P1 will introduce new bug workload**, and this is the most easily skipped step with the excuse
  "we have to fix bugs so let's not do it first" — but skipping it means all subsequent stages'
  acceptance criteria are built on false coverage.
- **P4 is the largest single-point risk**: one change to 5 entry points.
- **P6 has the widest impact**: touches the common dependency of parser, formatter, spawn,
  orchestrator.
- **P7's line count will net increase by 900-1600 lines, not net decrease**. Splitting introduces
  boilerplate, `verify` 300-600 lines, register allocation 600-1000 lines. **If "code gets shorter"
  is set as the success criterion at project launch, this stage will be judged a failure** — this
  must be aligned at project launch.

## Alternatives

### A. Only split files, don't change the paradigm

Split 8448 lines into 10 files of 800 lines each.

**Not adopted.** Splitting does not eliminate any root cause: 6 hand-written save/restore locations
are still scattered across 10 files, `generate_call_expr_ir`'s `arg_regs` semantic reshuffle still
requires manual reasoning, "adding an operator changes a dozen places" is still a dozen. Splitting
improves navigation, not correctness.

### B. Only add gates, don't change structure (including line-count ratchet)

**Not adopted as the only solution, but it is part of this RFC.** Gates can prevent degradation, but
cannot fix the current state — `ir_gen.rs` already has 8448 lines, gates can only freeze it at this
number.

### C. Start from scratch, write a new compiler skeleton

**Not adopted.** `src/package/` proves this team can get complex systems right. The problem is not
capability. And the existing 293 `.yx` corpus files provide a ready-made equivalence criterion
source, starting from scratch wastes this asset.

### D. Use an existing compiler framework (e.g., LLVM as the only backend)

**Not adopted.** `Executor` trait (`backends/mod.rs:356`) has reserved an abstraction point but only
one implementer. Introducing LLVM is the scope of `RFC-018` (1037 lines design, 0 lines code), and
should be **decided independently** — it changes "what to compile to" rather than "how to organize
code".

## Decision register: all open items decided

> **This section is the sole authority.** The original `- [ ]` open questions and "conflict
> registrations" in each companion document **all converge to this table**, no longer individually
> retained. **There is no "to be determined", no "deferred", no "optional".** Each entry gives the
> decision and reason. If during implementation it is found that the technical premise of a decision
> no longer holds, **the correct action is to return to this table to change the decision and
> explain the reason**, not to bypass it.

### Directories and naming

| #   | Issue                                                                                                                                                                              | **Decision**                                                                                                                                                   | Reason                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Whether to do directory renames (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`, move `parser/ast.rs` to top-level `src/ast/`, migrate opcode vocabulary to `middle/bytecode/`) | **Do it.** No phasing, no "optional", complete together with P5/P6 (AST top-level domain and bytecode domain merge in the same batch, both C1 pure relocation) | Boundaries are guaranteed by CI, but **directory names are the first-glance signal of responsibility**. `middle/core/` contains three things: `ir.rs` + `ir_gen.rs` + `bytecode.rs`, the name has already failed; AST is consumed by all layers, placing it under `frontend/` will mislead downstream paths (industry mainstream 6/7 vendors place it as a top-level domain parallel to parser) |
| D2  | Naming convention                                                                                                                                                                  | Keep `type_.rs` / `fn_.rs` (avoid Rust 2024 reserved words); forbid `xxx_v2` / `xxx_new` directories                                                           | Parallel copies are a common source of dead code (`Precedence` enum in `pratt/precedence.rs` parallel to actually-effective BP constants, zero production references; `Expr::FnDef` is a never-executing parallel construction path)                                                                                                                                                            |

### Stage contract (02)

| #   | Issue                                                                                                                                                                                                                                              | **Decision**                                                                                                                                        | Reason                                                                                                                                                                                                |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D3  | `check_module` (`typecheck/mod.rs:81`, fail-fast; early exit implementation in `inference/statements.rs:2797-2799`) vs `check_module_collect_all` (`typecheck/mod.rs:93`, collect-all; call sites `orchestrator.rs:124` / `:493`) whether to merge | **Do not merge.** Keep two APIs, driven by `Program`'s aggregation mode field (`Aggregation: FailFast \| CollectAll`, see 02)                       | The two semantics are indeed different (`CollectAll` serves LSP). Forcing a merge will introduce `Option` noise                                                                                       |
| D4  | Which of the 16 fields of `Obligations` are "must be consumed"                                                                                                                                                                                     | **All 16 go into the ledger.** Fields with no consumer (like `module_namespaces`) are deleted together in P4's S4, **no orphan fields remain**      | The meaning of the ledger is no exceptions                                                                                                                                                            |
| D5  | Diagnostic level of `assert_drained()`                                                                                                                                                                                                             | **Final E level** (error); within P4 first observed online as W level, promoted to E at the end of the same stage (the two-step approach in 02 §S4) | Staying at W long-term equals no gate — W does not change exit code; but one-step promotion to E will make C2 criteria unusable, so E is the terminal state, W is just a transitional state within P4 |
| D6  | `ownership.rs:627` creates a new Z3Backend every time                                                                                                                                                                                              | **Change to singleton**, unified with the global `LazyLock` of `predicate.rs:34-36` into `SolverProvider`                                           | Three acquisition strategies, two failure philosophies (one panic, one silent) must converge                                                                                                          |
| D7  | `layers/README.md` layer order is opposite to actual                                                                                                                                                                                               | **Fix it**, assigned to P4's 4.4.1                                                                                                                  | README describes an unbuilt intent architecture; fixing it will expose the previously silenced `Unproven`, **which should be exposed**                                                                |

### Type representation (03)

| #   | Issue                                                                                          | **Decision**                                                                                                                                                                             | Reason                                                                                                                                        |
| --- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| D8  | Whether to delete `AssocType` (`ast.rs:463-471`)                                               | **Delete.** Zero production construction (the only construction in the repo is in test `types/tests/mono.rs:178`, does not change the conclusion), independently re-checked with T1 gate | Same handling as the other 11 zero-construction variants; if associated type syntax is enabled in the future, a new proposal will redefine it |
| D9  | T1 gate uses `syn` or Python                                                                   | **Use `syn`.** `tools/code-tables` is already a Rust crate, reuse it                                                                                                                     | Building another Python parser is a duplicate implementation, violating prohibition 1                                                         |
| D10 | What does `Type::Void` fallback (`parser/statements/types.rs:836`) change to                   | **Change to `Err(Diagnostic)` return**                                                                                                                                                   | Silent fallback is a form of "fabrication" — using a fake value to cover up absence                                                           |
| D11 | `NameKind` determination + LSP path probe                                                      | **LSP reuses `TypeEnvProbe`**, no separate probe channel                                                                                                                                 | `lsp/world.rs` already holds `SemanticDB`, low reuse cost                                                                                     |
| D12 | `const_data::BinOp` / `ast::BinOp` who generates whom                                          | **Take `ast::BinOp` as the source**, `const_data` side aligns semantically (no forced rename `Neq`/`Ne`)                                                                                 | AST is the main representation; renaming affects 12 references with no benefit                                                                |
| D13 | `classify_generic_params` uses `signature_params` or `Lambda.params`; same as `ir_gen.rs:1380` | **Not an open issue, a mandatory verification item for P6 stage 6.1.3**, conclusion written into PR                                                                                      | This is an execution step, not an open policy                                                                                                 |
| D14 | Statistics of non-canonical type names in 293 corpus                                           | **Same as above is a task** (6.1.1), not an open issue                                                                                                                                   | Same as above                                                                                                                                 |
| D15 | Stage 2 "no behavior change" vs "behavior change" classification                               | **Classify per C3 criterion: design changes go in independent commits, wording changes not counted**                                                                                     | Already defined in 07                                                                                                                         |

### Intermediate representation (04)

| #   | Issue                                                                       | **Decision**                                                                                                                                                                           | Reason                                                                                                                                                                                                                                                                    |
| --- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D16 | `Instruction::Phi` static selection                                         | **Option A: IR-only, `codegen` expands into Move sequence**                                                                                                                            | Do not add a new `PHI` opcode, avoid `.42` format expansion                                                                                                                                                                                                               |
| D17 | `bytecode.rs:2312` `upvalue_count: 0`                                       | **Fix; bump `.42` version from 4 to 5 when adding fields** (format header already has `MAGIC` + `VERSION: u32` fields, `codegen/bytecode.rs:14-16`, reading side validates by version) | This is a data loss defect not dead code; after version bump old `.42` is rejected by reading side per existing behavior (`.42` is a build product, cross-version compatibility is not a goal); already-allocated unused opcode values handled together with version bump |
| D18 | Whether u8/255 slot limit is sufficient after deletion                      | **Must be verified at end of batch d**; if insufficient, expand to `u16`                                                                                                               | This is a verification item not an open item                                                                                                                                                                                                                              |
| D19 | Whether linear scan register allocator should first make a prototype timing | **Do not make a prototype, implement directly.** Time it in CI                                                                                                                         | Timing does not change design decisions, it's procrastination                                                                                                                                                                                                             |
| D20 | `ReleasePlan` / `overload_resolutions` Span keying                          | **Change to `PlanId`**, eliminate cross-layer span contract; assigned to P4                                                                                                            | Span mismatch causes Drop silent loss, one of the most hidden defect categories                                                                                                                                                                                           |
| D21 | `method_def_ordinals` belongs to 03 or 04                                   | **Belongs to 04** (P7 batch c)                                                                                                                                                         | It's IR construction-time state, not a type problem                                                                                                                                                                                                                       |
| D22 | Whether `synth.rs` boundary generalizes to all of L3                        | **Generalize to all of L3**; L4 to be investigated and included later                                                                                                                  | Boundary rules are either complete or invalid                                                                                                                                                                                                                             |
| D23 | Whether `compile_pattern` / `eval_const_expr` is "not handled"              | **Include in batch d scope**, not exempt                                                                                                                                               | Using "marked as not handled" to dodge work is patching-style thinking                                                                                                                                                                                                    |

### Frontend (05)

| #   | Issue                                                                             | **Decision**                                                                                                                                                  | Reason                                                                                                                                                                      |
| --- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D24 | Whether LALRPOP can fully reproduce existing diagnostics                          | **Must reproduce.** C5 is not relaxed. Use explicit error productions to write the `synchronize()` synchronization point set and skip timing into the grammar | Inability to do so is an implementation defect, **not a reason to relax the criterion**; honestly report and re-evaluate (including "keep Pratt"), not change the criterion |
| D25 | f-string span changing because it's determined at lexing time, is that acceptable | **Not acceptable.** Span must record **absolute offset** at lexing time, differential comparison of `span.file/line`                                          | Span change will affect diagnostic positioning, unacceptable                                                                                                                |
| D26 | Whether associativity needs explicit declaration                                  | **Needed.** Grammar explicitly declares left/right associativity, not relying on `bp_right = bp_left + 1` convention                                          | Current convention is the source of magic numbers like `BP_RANGE` hardcoded `(6,7)`                                                                                         |
| D27 | `is_old_function_syntax` (36 lines) handling                                      | **Option 1: naturally delete as grammar migrates.** `f(Int) -> Int = ...` cannot match any production                                                         | Specifically probing removed syntax is an anti-pattern                                                                                                                      |
| D28 | Whether stage 0 deletes `pub use precedence::*;` at `pratt/mod.rs:11`             | **Delete**                                                                                                                                                    | It is the exposure surface of the 96-line dead ladder                                                                                                                       |
| D29 | `parse_assign_after_target`'s 8-segment responsibilities                          | **All split, none retained.** `apply_semantic_side_effects` handed off to P6 but **not deleted**                                                              | Splitting is responsibility re-division, not deletion of responsibilities                                                                                                   |
| D30 | Stage 2-4 full corpus timing                                                      | **P2 baseline stage actually measured and recorded**                                                                                                          | Verification item not open item                                                                                                                                             |

### Cleanup and criteria (06 / 07 / 08)

| #   | Issue                                                         | **Decision**                                                                                                                                                                                                                                                           | Reason                                                                                                          |
| --- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| D31 | Which PR fixes defects exposed by S1's revived tests          | **Same PR**, test revival and defect fix together                                                                                                                                                                                                                      | Splitting will induce future "fix it next time"                                                                 |
| D32 | `Switch` opcode (interpreter has implementation, no producer) | **Delete the opcode and the interpreter implementation.** Handled together with `.42` version bump (D17)                                                                                                                                                               | Opcode with no producer is a dead path, whitelist just covers it up                                             |
| D33 | `UnaryOp::Not` silently degrades to `I64_NEG`                 | **Fix.** `opcode()` must distinguish the `op` field, cannot ignore                                                                                                                                                                                                     | Silently changing semantics is one of the most dangerous categories                                             |
| D34 | `Instruction::TailCall` encoding branch                       | **Delete** (no construction points repo-wide)                                                                                                                                                                                                                          | Same reasoning as D32                                                                                           |
| D35 | 27 wasm files' cfg branches                                   | **Delete unreachable ones in playground scenario, keep reachable ones.** Assigned to P10                                                                                                                                                                               | wasm target is already built (independent shim crate), branches are load-bearing, only delete unreachable parts |
| D36 | Whether `TRACKING.md` adds an "Implementation Status" column  | **Add.** `check_tracking.py` adds generation logic                                                                                                                                                                                                                     | Currently 26 accepted RFCs do not record implementation status at all, RFC-018 is how it rotted                 |
| D37 | `docs/superpowers/` § dead references (gitignored document)   | **Delete comments**                                                                                                                                                                                                                                                    | Pointing to a gitignored planning document is a dead reference                                                  |
| D38 | If `verify_loose` cannot run green                            | **No exemption.** If it cannot run green, it means `ir_gen` has an implicit "same slot multiple writes" dependency, **which is a defect that must be fixed first**                                                                                                     | Exemption list is new technical debt, equivalent to using gates to cover up design problems                     |
| D39 | Snapshot baseline size and compression                        | **Do not introduce git-lfs, accept the size**                                                                                                                                                                                                                          | Snapshots are regression criteria, readability takes priority over size                                         |
| D40 | Multi-file corpus layer                                       | **Unconditionally required** (P2's 2.3.2), not "skip otherwise P4 defers"                                                                                                                                                                                              | Conditional statements leave backdoors for oneself                                                              |
| D41 | Whitelist for `test_release_plan_spans_consumed`              | **No whitelist, difference set must be empty** (this test is new for P2; the repo's existing `WHITELIST = ["SWITCH"]` is for opcode roundtrip tests, unrelated to this item). If not empty, it means the ReleasePlan contract has a defect, fix the contract (see D20) | Whitelist legitimizes bugs                                                                                      |
| D42 | `verify()` hooks `cargo test` or CI                           | **CI**                                                                                                                                                                                                                                                                 | Full corpus validation timing is unsuitable for every local run                                                 |
| D43 | Similarity algorithm for prohibition 1 A criterion            | **Jaccard ≥ 0.5**                                                                                                                                                                                                                                                      | Already defined in `08` body, remove open issue                                                                 |
| D44 | Whether PRs must declare responsibility attribution           | **Yes.** Add required field to PR template                                                                                                                                                                                                                             | Responsibility determination is not machine-checkable, can only be enforced by process                          |
| D45 | Who reviews `// reason:` exemptions                           | **Merge into PR review checklist**, no dedicated person                                                                                                                                                                                                                | With no team division, setting a dedicated person equals not setting one                                        |

### Project and process

| #   | Issue                                                             | **Decision**                                                                                                                                                                                                       | Reason                                                                                                                                                                                                                                                                         |
| --- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D46 | `RFC-018` (accepted, 0 lines of code)                             | **Move back to `draft/`**                                                                                                                                                                                          | Per `rfc/index.md:131` definition "accepted = enters implementation phase", it has not                                                                                                                                                                                         |
| D47 | Stage and Issue mapping                                           | **Open one Issue per first-level stage**, register in `check_tracking.py`'s `issues_impl`                                                                                                                          |                                                                                                                                                                                                                                                                                |
| D48 | Multi-file corpus layer location                                  | **`tests/yaoxiang-multifile/`** (new)                                                                                                                                                                              | Extending `multifile.rs` would mix semantic tests with contract corpus                                                                                                                                                                                                         |
| D49 | Docs site navigation                                              | **Add.** Add a new section next to "Tool Design" in `config.js:255`                                                                                                                                                | Without it, the nine companion documents are unreachable                                                                                                                                                                                                                       |
| D50 | Stage parallelism (multi-file typecheck)                          | **Not in this round.** As an independent issue after criteria stabilize                                                                                                                                            | Parallelism will mask order-dependency defects, conflicting with this round's investigation goal                                                                                                                                                                               |
| D51 | P3 stop-bleeding channel                                          | **Allowed.** P3 prerequisites narrowed to 2.3.2 + 2.4.1 + 2.4.3; rest of P2 (IR validator / snapshot / single-file diff / performance baseline) can run in parallel with P3; P4 must wait for P2's full completion | Stopping the bleeding of correctness vulnerability (`Sorted(3)` silent pass) must not be blocked by snapshot infrastructure construction; P4 needs all three layers of criteria in place so not relaxed                                                                        |
| D52 | `.42` three hardcoded discards (`bytecode.rs:2312`/`2315`/`2341`) | **All consolidated into P7 (7e), no "independent issue" left.** `2315` (exception table) and `2341` (global variables) were originally independent issues, now consolidated                                        | Exception table loss makes throw/try behave incorrectly when `.42` runs directly — throw/try are language core semantics; this refactor leaves no to-be-implemented remnants of core functionality. Three same-fix only needs one `VERSION` bump (4→5), cheaper than two bumps |
| D51 | P3 stop-bleeding channel                                          | **Allowed.** P3 prerequisites narrowed to 2.3.2 + 2.4.1 + 2.4.3; rest of P2 (IR validator / snapshot / single-file diff / performance baseline) can run in parallel with P3; P4 must wait for P2's full completion | Stopping the bleeding of correctness vulnerability (`Sorted(3)` silent pass) must not be blocked by snapshot infrastructure construction; P4 needs all three layers of criteria in place so not relaxed                                                                        |
| D52 | `.42` three hardcoded discards (`bytecode.rs:2312`/`2315`/`2341`) | **All consolidated into P7 (7e), no "independent issue" left.** `2315` (exception table) and `2341` (global variables) were originally independent issues, now consolidated                                        | Exception table loss makes throw/try behave incorrectly when `.42` runs directly — throw/try are language core semantics; this refactor leaves no to-be-implemented remnants of core functionality. Three same-fix only needs one `VERSION` bump (4→5), cheaper than two bumps |

### Sole data item to be added

| #   | Item                | Description                                                                                                                                                                                  |
| --- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| —   | RFC's `issue` field | Currently `TBD`. **A real Issue number must be added before submitting a PR**, otherwise `check_tracking.py` may pass, but `scripts/rfc/ai_agent.py`'s `issue_rfc_link` cannot be associated |

## Appendix: Glossary

| Term                                       | Definition                                                                                                                          |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **Stage**                                  | An exhaustively enumerable step in a compilation. The set of stages must be an exhaustively enumerable enum, not runtime-registered |
| **Orchestration layer (L1)**               | The layer that decides which stages run, how many times, and how failures propagate                                                 |
| **Frontend layer (L2)**                    | Lexing and parsing, producing AST                                                                                                   |
| **Intermediate representation layer (L3)** | Type checking, static analysis, IR construction                                                                                     |
| **Execution layer (L4)**                   | Bytecode generation, interpreter, runtime, standard library                                                                         |
| **Obligation**                             | A contract item produced by a stage that must be consumed downstream or compilation fails                                           |
| **Obligations ledger**                     | A container of all obligations, settled uniformly at the end of the stage table                                                     |
| **Orphan**                                 | A file never referenced by any `mod` declaration, therefore never participating in compilation                                      |
| **Shell**                                  | A file already declared, entering the compilation product, but containing only doc comments or unused `use`s, zero assertions       |
| **Equivalence oracle**                     | Executable checks proving behavior unchanged before and after refactoring                                                           |
| **C1-C6**                                  | Refactoring category grading, determining criterion strength. **No C5′**                                                            |
| **Separation of responsibilities**         | The only solution to scale problems: one module takes only one category of responsibility. **No line count / volume gate**          |
| **Decision register**                      | The "Decision register" section of this RFC, the final adjudication of all open items                                               |

## Appendix: Design decision log

| Decision                 | Decision                                                                                                                                 | Date       | Recorder  |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------- |
| Layering granularity     | Four layers (L1-L4)                                                                                                                      | 2026-10-03 | ChenXu233 |
| Document structure       | **1 RFC + 9 companion design documents** (`docs/src/design/compiler-architecture/`, same level as `check/`, `formatter/`)                | 2026-10-03 | ChenXu233 |
| Directory rename         | **Do it** (D1), no phasing                                                                                                               | 2026-10-03 | ChenXu233 |
| AST ownership            | **Top-level `src/ast/`**, parallel to parser (industry mainstream; check-boundary exempt clause)                                         | 2026-10-04 | ChenXu233 |
| Opcode ownership         | **`middle/bytecode/opcode.rs`**, merged with bytecode domain (vocabulary and format in same domain, dependency direction restored L4→L3) | 2026-10-04 | ChenXu233 |
| Scale gate               | **All cancelled.** Scale solved by separation of responsibilities, adopt Go's official position                                          | 2026-10-03 | ChenXu233 |
| Grammar paradigm         | **Full LALRPOP grammar driven**; reject the middle ground of "keep Pratt and table-ize"                                                  | 2026-10-03 | ChenXu233 |
| Equivalence oracle       | **C1-C6 six categories graded, no relaxation whatsoever** (no C5′)                                                                       | 2026-10-03 | ChenXu233 |
| Deletion strategy        | Allow thorough deletion; orphan tests with real assertions **revive rather than delete**                                                 | 2026-10-03 | ChenXu233 |
| Verification method      | This set of documents does not execute cargo; based on static evidence + line numbers                                                    | 2026-10-03 | ChenXu233 |
| Stage order              | P0 maintenance mechanism → P1 revive tests → P2 criteria → … → P10 cleanup                                                               | 2026-10-03 | ChenXu233 |
| Open items               | **All 50 locked in** (decision register D1-D50), no trade-offs left                                                                      | 2026-10-03 | ChenXu233 |
| P3 stop-bleeding channel | **Allowed** (D51), prerequisites narrowed to 2.3.2+2.4.1+2.4.3                                                                           | 2026-10-05 | ChenXu233 |
| `.42` three data losses  | **All consolidated into P7 (7e), no independent issue left** (D52)                                                                       | 2026-10-05 | ChenXu233 |

## References

- [RFC-010 Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Direct source for type
  representation convergence
- [RFC-011 Generic Type System](../accepted/011-generic-type-system.md)
- [RFC-011a Interface Implementation](../accepted/011a-interface-implementation.md)
- [RFC-013 Error Code Specification](../accepted/013-error-code-specification.md) — Example of
  generation-time gate
- [RFC-027 Compile-time Evaluation and Types](../accepted/027-compile-time-evaluation-types.md)
- [RFC-029 Module Semantics](../accepted/029-module-semantics.md)
- [RFC-036 Test Framework](../accepted/036-test-framework.md)
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](029a-module-cache-incremental.md)
- `src/frontend/core/typecheck/layers/README.md` — Current layer order declaration (opposite of
  actual, D7 requires fix)
- `build.rs:19-55` — Existing `panic!`-level gate example in this repository
- `docs/src/design/check/` — Three documents describing a never-existing architecture (see
  `06-cleanup-inventory.md` §C)
