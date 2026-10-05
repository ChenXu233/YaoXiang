---
title: 'RFC-039: Compiler Architecture Refactor (Master Plan)'
author: 'ChenXu233'
created: '2026-10-03'
updated: '2026-10-04'
status: 'Draft'
issue: 'TBD'
---

# RFC-039: Compiler Architecture Refactor

## Summary

This RFC proposes an architectural refactor of the YaoXiang compiler (`src/` 160,002 lines / 509
`.rs` files): redraw boundaries using a **four-layer model** (orchestration / frontend /
intermediate representation / execution), establish an **exhaustible compilation stage contract**
and an **obligations ledger**, and pair them with a set of **equivalence oracles** and
**anti-regression gates**.

Detailed design lives in nine companion documents under `docs/src/dev/architecture/`. This RFC is
responsible only for: problem, root cause, four-layer model, acceptance-oracle grading, and
execution-stage order. **Reviewing this RFC alone is enough to decide whether to greenlight the
project; the companion documents are the construction blueprints after greenlight.**

## Motivation

### One, boundaries exist but have no enforcement

The directory-name layering (`frontend` / `middle` / `backends`) is clear. But **the boundaries live
only in human self-discipline — no mechanism prevents them from being eroded**. Three direct
consequences.

#### 1.1 Compilation stages are hand-wired at five entry points, producing 11 behavioral inconsistencies

| Stage                       | Single-file `run` | Multi-file `run` | `check` | LSP (in project) | LSP (single file) |
| --------------------------- | ----------------- | ---------------- | ------- | ---------------- | ----------------- |
| lexing / parsing            | Yes               | Yes              | Yes     | Yes              | Yes               |
| typecheck                   | Yes               | Yes              | Yes     | Yes              | Yes               |
| **proof_execution**         | **Yes**           | **No**           | **No**  | **No**           | **No**            |
| Dead code analysis          | Yes               | No               | Yes     | No               | No                |
| W1006 shadowing diagnostics | No                | No               | Yes     | No               | No                |
| Monomorphization            | Yes               | No               | No      | No               | No                |
| IR generation / linking     | Yes               | Yes              | No      | No               | No                |

The five entry points: `src/frontend/pipeline.rs:141-227` (single file, 5 stages at `149` / `161` /
`173` / `188` / `205`), `src/frontend/compiler.rs:106` (wrapper), four in
`src/frontend/module/orchestrator.rs` (`compile_project:99` / `check_project:273` /
`check_source_in_project:450` / `compile_embedded_module:1374`), plus `src/lib.rs`'s `run_file:140`
/ `run_project:154`.

The most severe cell causes a **correctness vulnerability**. A proof-function constraint shaped like
`x: Sorted(3)` **silently passes** under the multi-file, `check`, and LSP paths. Evidence chain (all
auditable):

1. `src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **sole** place in the whole repo
   that constructs a non-empty `proof_calls`, with the semantics "this constraint requires executing
   a proof function".
2. `src/frontend/core/typecheck/checker.rs` has three isomorphic branches handling `Unproven`:
   `5164` / `5306` / `5420` are `if calls.is_empty()` → push a hard error; `5179` / `5318` / `5448`
   are `ctx.proof_calls.extend(...)` → **no diagnostic is produced**. The comment at `5165` reads
   verbatim: "Unproven → compile error, no degradation, no silent pass... do not let silent pass
   come back to life".
3. `TypeCheckResult.proof_calls` (`types.rs:29`) is read in the entire repo at **only one place:
   `src/frontend/pipeline.rs:187`**.
4. The four entry points in `orchestrator.rs` **do not read this field**.

That is: the invariants the code declares for itself are broken by architecture, not by logic. The
existence of `compile_embedded_module` (`orchestrator.rs:1374`) means that **the refinement
obligations of embedded std modules themselves also travel through this discarding path**.

**No test can catch it**: of the 27 tests in `tests/integration/multifile.rs`, **zero** hit `Sorted`
/ `proof` / `refin`; RFC-027's unit tests directly call `check_module` asserting "`proof_calls` is
non-empty" — **stopping right before the pipeline** — they verify "it was filled", but the bug is
"the consumer side does not read it".

#### 1.2 Module boundaries can be bypassed without leaving a trace

`src/frontend/core/typecheck/checker.rs:5618` is `include!("checker/semantic_tokens.rs");` — **the
only `include!` in the whole repo**. It splices 1,547 lines of text into the `checker` module. This
file **is not declared as any module** (`grep 'mod semantic_tokens'` returns zero hits in the whole
repo), its first line is `impl TypeChecker {`, it has no `use` header of its own, and the `checker/`
directory contains only this one file.

Consequences: no module identity, visibility isolation fails, rust-analyzer's jump and symbol search
do not work for it, and toolchain analysis counts it as part of `checker.rs`. **The real size of the
`checker` module is 5618 + 1547 ≈ 7,165 lines**, not the 5,618 lines the directory statistics show.

#### 1.3 Test wiring relies on human memory, corruption happens silently

A BFS reachability analysis of the module graph from the two crate roots `src/lib.rs` /
`src/main.rs` shows: **24 files / 2,628 lines never participate in compilation**. Among these, 23
files / 1,081 lines are real orphans.

| Orphan                                      | Scale                | Content                                                                        |
| ------------------------------------------- | -------------------- | ------------------------------------------------------------------------------ |
| `frontend/core/lexer/tests/` entire subtree | 13 files / 686 lines | 7 shells totaling 19 lines + `mod.rs` 38 lines + **629 lines / 55 real tests** |
| `frontend/pipeline/tests/`                  | 3 files / 14 lines   | All placeholder doc comments                                                   |
| `package/template/tests/`                   | 3 files / 65 lines   | 7 `#[test]`                                                                    |
| `parser/pratt/tests/precedence_inline.rs`   | 95 lines / 6 tests   | Hidden in a live directory                                                     |
| `typecheck/passes/tests/overload_inline.rs` | 170 lines / 7 tests  | Same                                                                           |
| `util/diagnostic/emitter/tests/json.rs`     | 46 lines / 3 tests   | Same                                                                           |

`cargo test` cannot discover this: a test that never runs does not fail. CI
(`.github/workflows/ci.yml:140`) has no test-count baseline or wiring check either.

**A total of 1,005 lines / 78 tests have never run.** The only thing actually running at the lexer
layer is `fstring.rs` (slipped in via the `#[path]` attribute bypass).

### Two, design capability is sufficient

It must be acknowledged simultaneously: this project **already has** mature enforcement mechanisms —
they just have not been generalized.

The error codes in `src/util/diagnostic/codes/` (137 E codes + 8 W codes = 145) are run through a
build-time hard gate by `build.rs:19-55` via `tools/code-tables` — parsing the registry, validating
uniqueness and section placement, and comparing each entry against RFC-013's markdown code table.
**Any inconsistency causes `panic!` and refuses compilation**. As a result, RFC-013's documentation
and the code are always in sync.

`src/package/` (76 files / 13,012 lines) is the most complete region: **tests are 6,227 lines,
47.9%**; 5 of the 6 `tests/` directories are correctly wired (the only exception is
`template/tests/`, see Section A). Note that this positive sample holds at the **directory-level
wiring**, not at the per-file level — out of 76 `.rs` files, only 7 contain inline `#[cfg(test)]`
and 5 contain `mod tests;`. Five RFCs match the code item by item, and many "not implemented"
markers have become explicit errors rather than silent TODOs (cmake at `build/mod.rs:172-174`,
`RegistryDeferred` at `error.rs:74-76`). The `.yxpkg` has five layers of safety protection, all
correct.

**Same repo, same author: `package/` got test-directory wiring right, while `lexer/tests/` has never
run a single time. The difference is not in capability, but in whether someone ran a check once.**

### Three, vapor design

The three documents under `docs/src/dev/design/check/` describe an architecture that **never
existed**: `CheckSession` (including the Rust code draft at `incremental-checking.md:31-41`),
`ModuleDependencyGraph`, `ModuleCache`, and `HotReloader` all return zero hits in the whole repo;
the `traits/` directory does not exist; the `check_single_module` function does not exist (it lives
only in test comments); even the `command.rs` busy-wait referenced under "known limitations" does
not exist (that file has neither `Instant` nor `recv_timeout`).

The actual cross-file analysis is implemented by `src/frontend/module/` (registry 439 + resolver
195 + roles 388 + orchestrator 1528, roughly 2,670 lines) — the capability is real, the path is
entirely different. **This documentation wrote "possible future design" as "deficits of an
already-implemented system".**

`src/frontend/pipeline/tests/compilation_cache.rs` and `incremental_scheduler.rs` (3 lines each) are
the cleanest physical evidence: "built a test skeleton to design, found no corresponding
implementation in the pipeline, then set it aside".

Additionally, `RFC-018` (LLVM AOT, 1,037 lines, status accepted) has 0 lines of implementation in
the code — all 6 AOT-related hits in `backends/mod.rs` are **comments**.

## Root Cause Diagnosis

Combining the three symptom groups, the root cause is singular:

> **This project treats "design" as documentation convention, not as an executable constraint.**

| Layer           | Missing enforcement                                                 | Existing but ungeneralized example                                           |
| --------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Stage boundary  | A single decision point for "which stages did this compilation run" | —                                                                            |
| Module boundary | Forbid `include!`, limit `pub(crate)` cross-layer leakage           | —                                                                            |
| Cross-reference | Detectability of "field produced but no consumer"                   | The error-code gate in `build.rs`                                            |
| Test wiring     | Declaring a directory requires a `mod` declaration                  | `src/package/`'s directory-level wiring convention (5 of 6 `tests/` correct) |

This RFC's position is therefore: **turn the three classes of boundary — stages, modules,
cross-references — from "documentation convention" into "compilable or testable facts"**, and
**establish a single stage decision point in the orchestration layer**.

## Proposal

### Four-Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│ L1 Orchestration                                            │
│   Who decides which stages to run, how many times,          │
│   how failures propagate                                    │
│   Current: pipeline.rs(740) + orchestrator.rs(1528)         │
│         + compiler.rs(267) = 5 entry points, 11 mismatches  │
│   Target:  Single Driver + exhaustible Stage enum           │
│         + Obligations ledger                                 │
│   See: ../compiler-architecture/02-stage-contract.md           │
└───────────────────────────┬─────────────────────────────────┘
                            │ Compilation unit (Unit)
┌───────────────────────────▼─────────────────────────────────┐
│ L2 Frontend                                                 │
│   Lexing, parsing, AST construction                          │
│   Current: lexer/(3065) + parser/(6341)                      │
│         hand-rolled lexer + hand-rolled Pratt; adding an     │
│         operator touches 6-7 places inside L2 plus 17       │
│         downstream production files                         │
│   Target:  lexer converges to a single implementation +      │
│         LALRPOP grammar-driven syntax                        │
│   See: ../compiler-architecture/05-frontend-paradigm.md        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Module (AST)
┌───────────────────────────▼─────────────────────────────────┐
│ L3 Semantic & IR                                            │
│   Type checking, static analysis, IR construction            │
│   Current: typecheck/(29466 production) + ir.rs(905)         │
│         + ir_gen.rs(8448)                                    │
│         3 parallel type representations +                    │
│         3 parallel operator enums;                            │
│         ir_gen single impl 7952 lines / 116 methods;         │
│         no IR-level verifier                                │
│   Target:  unify type representation; IR satisfies           │
│         SSA construction discipline                          │
│   See: ../compiler-architecture/03-type-unification.md         │
│         ../compiler-architecture/04-ssa.md                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ ModuleIR
┌───────────────────────────▼─────────────────────────────────┐
│ L4 Execution                                                │
│   Bytecode generation, interpreter, runtime, stdlib          │
│   Current: codegen/(3436) + bytecode.rs(2422) +              │
│         executor/(2000) + runtime/(1800) + std/(7543)        │
│   Target:  opcode single source of truth + generation-time    │
│         gate; L4 is unaware of L1-L3                          │
│   See: ../compiler-architecture/06-cleanup-inventory.md        │
└─────────────────────────────────────────────────────────────┘
```

**The four layers are strictly one-way**: L1 depends on L2/L3/L4's interfaces; L2 does not depend on
L3; L3 does not depend on L2 (it only consumes AST data, does not call parser); L4 only depends on
L3's output format.

The known reverse dependencies and parallel-definition list live in the routing table C of
`../compiler-architecture/01-routing.md`.

### Companion Design Documents

Nine, stored at `docs/src/dev/architecture/`:

| Document                          | Layer            | Topic                                                                                                                                                                                                                                  |
| --------------------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `01-routing.md`                   | Cross-layer      | **Feature routing table**, dependency-direction rules, **target directory structure after construction**, future-extension guide                                                                                                       |
| `02-stage-contract.md`            | L1               | Stage contract and obligations ledger: 11 mismatches, vulnerability evidence chain, `Stage` / `Obligations` / `Driver` / `ProgramKind` (including `WasmPlayground` on the wasm side)                                                   |
| `03-type-unification.md`          | L2/L3 root cause | Unify type representation: handling each of the 26 `ast::Type` variants, 13 forward-zero-constructible variants, type-table gate                                                                                                       |
| `04-ssa.md`                       | L3               | IR SSA-ification: four defect classes, SSA shape definition, register allocator evaluation, 38-item change list                                                                                                                        |
| `05-frontend-paradigm.md`         | L2               | Lexer converges to a single implementation / syntax becomes LALRPOP-grammar-driven, change surface converges, dead-staircase handling, test rebuild                                                                                    |
| `06-cleanup-inventory.md`         | Global           | Dead code and vapor-design cleanup: reachability method, per-item handling, execution order                                                                                                                                            |
| `07-equivalence-oracle.md`        | Cross-layer      | Equivalence oracle: three-layer oracle, C1-C6 grading, gate design                                                                                                                                                                     |
| **`08-maintenance-mechanism.md`** | **Cross-layer**  | **Repository maintenance mechanism**: three prohibitions (no invention / no endless padding / patches in place of refactors), D0–D4 decision procedure, machine-checkable rules, code-review checklist, external convention references |
| **`09-execution-wbs.md`**         | **Cross-volume** | **Multi-level construction work-breakdown table**: 11 first-level / 45 second-level / 122 third-level tasks, dependencies and parallel groups, **8 conflict registrations**                                                            |

`01` and `09` are **long-term reference documents** — the former does not become invalid with any
refactor, the latter is the construction checklist. The three prohibitions produced by `08`
constrain every action in P1–P10.

### Equivalence-Oracle Grading

**This is the safety net for all refactoring and must be established before any code change.**
Detailed design lives in `07-equivalence-oracle.md`.

Core principle: **oracles are graded by refactor category, because different categories need
different strengths of equivalence**. Using a single oracle (usually "IR snapshot equality") to
cover all refactors is wrong — some refactors **necessarily change IR shape**.

| Category | Refactor content                                                 | Oracle type                                                    | Strength  |
| -------- | ---------------------------------------------------------------- | -------------------------------------------------------------- | --------- |
| **C1**   | Pure moves (split files, change directories, extract submodules) | Normalized IR snapshot **zero-diff**                           | Strongest |
| **C2**   | Orchestration change (staging, unifying Driver)                  | Same diagnostic set at each entry + same corpus behavior       | Strong    |
| **C3**   | Type-representation convergence                                  | Diagnostic **codes** are the same (message wording may change) | Strong    |
| **C4**   | IR shape change (SSA-ification)                                  | **Behavioral equivalence** + IR structure invariants           | Medium    |
| **C5**   | Frontend paradigm change                                         | AST snapshot + diagnostics + behavior, all three               | Strong    |
| **C6**   | Pure deletion                                                    | No equivalence needed, only confirm no references              | —         |

**The distinction between C1 and C4 is key**: C1 requires IR to be byte-for-byte identical; C4
acknowledges that IR will change, and instead uses "same program behavior + IR satisfies
invariants". Forcibly using snapshots for C4 will tempt the team to relax the oracle.

Three-layer oracle:

1. **IR static verifier** — dominant use-before-def, jump targets exist, type consistency,
   inner-layer isolation. Far stronger than snapshots, covers defects like "IR is self-consistent
   but values are wrong" that snapshot tests cannot catch. Must **run green on the existing
   (non-SSA) IR first**.
2. **Normalized IR snapshot** — strip `Span`, rename temporaries by occurrence order, relativize
   global slots, sort predecessors. Check in, manually review diff. **Known limitation**:
   normalization erases "which register the Nth argument used", therefore **it alone cannot cover
   `arg_regs` semantic reordering**.
3. **End-to-end corpus diff** — diagnostics list, exit code, stdout/stderr compared item by item.

> **⚠️ Structural limitation of corpus coverage**: there is **no `yaoxiang.toml` at all** under the
> `tests/` directory (measured 0), therefore all **293** `.yx` corpus files under `tests/yaoxiang/`
> go through the single-file path (`check_files_with_diagnostics`'s standalone branch →
> `check_single_file` → `Pipeline::run`).
>
> **Consequence**: the third-layer oracle **can only validate single-file path changes**. For the
> multi-file side (the four entry points of `orchestrator`) and the multi-file behavior of
> types/SSA, **there is currently no corpus coverage at all**. P4 "Unified Driver" is the largest
> risk point of the whole plan; **the multi-file corpus layer must unconditionally be established in
> P2** (resolution D40) — it is the only executable source of behavioral oracle for P4.

## Implementation Strategy

### Phase Sequence

```
P0  Repository maintenance mechanism and code-placement rules 08   ← Rules first: three prohibitions + D0-D4 decision procedure
P1  Revive orphan tests and fix defects              06 §S1   ← Cheapest, highest payoff
P2  Establish equivalence-oracle baseline            07 all
P3  Fix the correctness vulnerability (minimal)     02 §Fix
P4  Stage contract and unified Driver                02
P5  Split checker inside its file                   09 §P5 fill-in
P6  Unify type representation                        03
P7  IR SSA-ification                                 04     } parallelizable
P8  Frontend paradigm change                         05     }
P9  Anti-regression gates (unified script list)      01 + 08
P10 Remaining cleanup and status corrections         06 §S2-S6
```

**Core serial constraints (non-swappable):**

| Constraint   | Reason                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1      | The three prohibitions constrain every action in P1 — 1.2.x is precisely adding `mod` declarations to `mod.rs`, which is exactly how `precedence_inline.rs` got its life-and-death conflict                                                                                                                                                                                             |
| P1 → P2      | Reviving tests changes the test count and corpus baseline. Building the baseline first would become invalid after P1                                                                                                                                                                                                                                                                    |
| P2 → P3      | The vulnerability oracle must be **written red first** to prove the fix works. **Narrowing (D51)**: P3's prerequisites are only 2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red oracles); the rest of P2 can run in parallel with P3, but P4 must wait for P2 to complete in full — the correctness-vulnerability hemostasis is not blocked by snapshot-infrastructure build-out |
| P3 → P4      | Fix the bug first, then refactor. Reversing this order would let the bug be solidified into "established behavior" by the stage table                                                                                                                                                                                                                                                   |
| P4 → P5      | P5 opens with `include!` → real `mod` (09 §P5 5.1), the subsequent checker split touches the same file and must proceed continuously after P4                                                                                                                                                                                                                                           |
| P5 → P6      | Consecutive changes to the same file must be split, otherwise regressions cannot be bisected                                                                                                                                                                                                                                                                                            |
| P6 → P7 / P8 | If the type representation is not unified first, SSA's new IR will grow into a third representation                                                                                                                                                                                                                                                                                     |
| P7/P8 → P9   | The initial baseline of each gate script (e.g., `pub(crate)` leak count) takes its final value only after P7/P8                                                                                                                                                                                                                                                                         |
| P7 ∥ P8      | Disjoint file sets, parallelizable                                                                                                                                                                                                                                                                                                                                                      |

**Why P0 is a separate phase**: P9's CI scripts are **post-hoc checks** that can only block bad code
that has already been written; "should have refactored but patched instead" is a **pre-hoc
judgment** that only the D0–D4 decision procedure can block. The two cannot replace each other.

### Per-Phase Essentials

| Phase   | Touch scope                                                                                                                                                                                                       | Acceptance                                                                                                                                                                                                     | Rollback point                                                                                  |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **P0**  | `CONTRIBUTING.md` rules section + 3 new gate scripts (`check-concepts` / `check-fanout` / `check-boundary`); **no compiler source changed, no line-count gate**                                                   | `check-concepts.py` **must report 3 operator enums and 2 parallel type representations (counting `ir::Type` aliases) on untouched code**; intentional over-limit / adding a 6th entry-style wiring must go red | Pure addition, deleting the script suffices                                                     |
| **P1**  | 5 files add `mod` declarations; **1,005 lines / 78 tests revived**                                                                                                                                                | Test count rises; **real defects expected to be exposed** (literal overflow paths in `literals.rs`, illegal `\x`/`\u` escapes never tested)                                                                    | Revert per file; **fixed defects should not be rolled back**                                    |
| **P2**  | New `verify.rs`, snapshot baseline, corpus-diff framework, **multi-file corpus layer**, performance baseline (criterion smoke benchmark), `scripts/ci/check-*`                                                    | `verify_loose` runs green; vulnerability oracle **written red**                                                                                                                                                | Oracle code can be removed in its entirety                                                      |
| **P3**  | `types.rs` (`proof_calls` made private), three consumption points in `orchestrator.rs`                                                                                                                            | Vulnerability oracle **turns green**; **intentionally removing the fix must turn it red again**                                                                                                                | Pure behavior fix                                                                               |
| **P4**  | New `src/driver/`; rewrite 5 entry points; **4.5: `ReleasePlan` / `overload_resolutions` keys `Span` → `PlanId` (D20)**                                                                                           | Same diagnostic set + same corpus behavior (C2); span-keyed silent failure reduced to zero                                                                                                                     | **Largest risk point**; keep old entry points; revert by leaving Driver unwired                 |
| **P5**  | `include!` → real `mod` (5.1); split `refinement` / `annotations` out of `checker.rs`                                                                                                                             | **IR snapshot zero-diff** (C1); `statements.rs:16`'s `pub(crate)` path unchanged                                                                                                                               | Revert per file                                                                                 |
| **P6**  | `ast.rs`, `types/mono.rs`, `solver.rs`, `ir.rs:3`, `bytecode.rs:2353-2390`; phase close-out executes directory renames (D1: `typecheck/`→`sema/`, `middle/core/`→`middle/ir/`)                                    | Diagnostic **codes** identical (C3); rename batches are pure moves (C1 zero-diff)                                                                                                                              | Submit in stages; variant handling and synonym-table deletion separated; rename owns its commit |
| **P7**  | `ir.rs`, `ir_gen.rs` (8,448 lines), `bytecode.rs`, `translator.rs`; **7e: three `.42` data-loss fixes (`upvalue_count` / `exception_handlers` / `globals`) + `VERSION` 4→5 (D17, D52)**                           | **Behavior equivalence + `verify_ssa` green** (C4). **Not** full IR snapshot equality; `.42` round-trip test                                                                                                   | Revert per batch                                                                                |
| **P8**  | `lexer/*`, `parser/*`                                                                                                                                                                                             | AST snapshot + diagnostics + behavior (C5)                                                                                                                                                                     | Revert per file                                                                                 |
| **P9**  | New `scripts/ci/check-*.py` totaling 10 (sole list at [09](../../dev/architecture/09-execution-wbs.md) §P9: 3 with P0, 1 with P1, 3 with P2, 2 with P4, 1 with P7 introduced, P9 closes and converts all to hard) | **Intentionally creating an obligation without a consumer must go red**; **intentionally creating an orphan test directory must go red**; **intentionally adding `include!` must go red**                      | Delete the script                                                                               |
| **P10** | Documentation disposal, RFC status correction, `pub` items downgraded in visibility, opcode decisions                                                                                                             | `check_tracking.py` passes; doc site has no 404                                                                                                                                                                | Independent PR                                                                                  |

### Definition of Done (DoD) per Phase

A phase is complete only when all of the following hold:

1. All "implementation strategy" steps in the belonging document are committed
2. The oracle for the corresponding category (C1–C6) in `07-equivalence-oracle.md` is **green in
   CI**
3. `cargo test` is fully green, and **the test count is not lower than when the phase started**
   (prevent "delete tests to turn green")
4. `cargo clippy --all --all-features -- -D warnings` is clean
5. `python scripts/rfc/check_tracking.py` exits 0
6. The phase has an independent revert unit — **if a phase cannot be reverted alone, it is too large
   and must be split again**
7. For phases that touch the compilation pipeline or execution path (P4 / P6 / P7 / P8), the 07
   performance baseline shows no unexplained >10% regression

### Global Acceptance Gates

After all phases are complete, the following must hold:

| #   | Condition                                            | Verification                                                                        |
| --- | ---------------------------------------------------- | ----------------------------------------------------------------------------------- |
| G1  | The decision point for compilation stages is unique  | Answering "which stages ran, which did not, why" requires looking at only one place |
| G2  | No "produced but never consumed" fields              | `check-obligations.py` green                                                        |
| G3  | Count of `include!` is 0                             | `grep -rn 'include!' src/`                                                          |
| G4  | No orphan tests                                      | `check-test-wiring.py` green                                                        |
| G5  | No reverse dependencies                              | `check-module-boundary.py` green                                                    |
| G6  | Type representation is unique                        | Parallel representations converge; hand-written synonym tables disappear            |
| G7  | Parser contains no type/predicate hardcoding         | Literals like `"Terminates"` are replaced by data flow                              |
| G8  | Adding a binary operator's change surface ≤ 3 places | Manual review + routing table update                                                |
| G9  | `layers/README.md` describes the actual layer order  | Documentation consistent with `check_module_impl`                                   |
| G10 | `TRACKING.md` has an "Implementation Status" column  | Implementation status of 52 RFCs is queryable                                       |

## Trade-offs

### Advantages

- **Provides a landing point for all subsequent changes**. "Adding an operator touches a dozen
  places, and no one notices when something is missing" cannot be improved by memory — only by a
  routing table + gate.
- **Singular root cause**. The three symptom groups (stage inconsistency, `include!`, test-wiring
  breakage) all stem from "boundaries are conventions, not constraints", and can be solved once with
  the same kind of mechanism.
- **Non-blocking**. This RFC makes zero code changes. The four-layer model is descriptive, and after
  adoption only a subset of the work may be executed.
- **Oracles first**. P2 is established before any code change, and the graded design avoids the
  common failure of "snapshot too strict → tempting to relax".

### Disadvantages and Risks

- **The serial chain is long**. P1 → P2 → P3 → P4 → P5 → P6 → P7/P8 is nearly fully serial, with
  later phases waiting on earlier ones.
- **P1 will introduce new bug-fix work**, and it is the step most easily skipped with "we'll have to
  fix bugs so let's not do it" — but skipping it leaves all subsequent phases' acceptance oracles
  built on false coverage.
- **P4 is the largest single-point risk**: it changes all 5 entry points at once.
- **P6 has the broadest impact surface**: it touches the shared dependencies of parser, formatter,
  spawn, and orchestrator.
- **P7's line count will net-add 900-1,600 lines, not net-subtract**. Splitting introduces
  boilerplate, `verify` is 300-600 lines, register allocation is 600-1,000 lines. **If the project's
  success criterion at greenlight is "code gets shorter", this phase will be judged a failure** —
  this must be aligned at greenlight.

## Alternatives

### A. Only split files, do not change paradigm

Split 8,448 lines into 10 files of ~800 lines each.

**Not adopted.** Splitting does not eliminate any root cause: 6 hand-written save/restore sites
remain scattered across 10 files, the `arg_regs` semantic reordering inside `generate_call_expr_ir`
still requires human reasoning, "adding an operator still touches a dozen places" is still a dozen
places. Splitting improves navigation, not correctness.

### B. Only add gates, do not change structure (including a line-count ratchet)

**Not adopted as the sole approach, but it is part of this RFC.** Gates prevent regression, they
cannot fix the current state — `ir_gen.rs` is already 8,448 lines, and a gate can only freeze it at
that number.

### C. Start over, write a new compiler skeleton

**Not adopted.** `src/package/` proves this team can build complex systems correctly. The problem is
not in capability. Moreover, the existing 293 `.yx` corpus files provide a ready-made source of
equivalence oracles; rewriting from scratch would waste that asset.

### D. Use a ready-made compiler framework (e.g., LLVM as the sole backend)

**Not adopted.** The `Executor` trait (`backends/mod.rs:356`) has an abstraction point reserved but
only one implementer. Adopting LLVM is the territory of `RFC-018` (1,037 lines of design, 0 lines of
code) and should be **decided independently** — it changes "what to compile to", not "how to
organize code".

## Decision Registry: All Open Items Resolved

> **This section is the sole authority.** All original `- [ ]` open items and "conflict
> registrations" in the companion documents **are all converged into this table**, no longer
> individually retained. **There is no "TBD", no "defer", no "optional".** Each entry provides a
> decision and rationale. If during implementation a decision's technical premise is found to no
> longer hold, **the correct action is to return to this table, change the decision, and explain
> why** — not to work around it.

### Directories and Naming

| #   | Topic                                                                                                                                                                      | **Decision**                                                                                                                                               | Rationale                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Whether to rename directories (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`, move `parser/ast.rs` to a top-level `src/ast/`, move opcode table to `middle/bytecode/`) | **Do it.** No staging, no "optional"; completed alongside P5/P6 (the top-level AST domain and bytecode domain merge is the same batch, both C1 pure moves) | Boundaries are CI-guaranteed, but **directory names are the first-glance signal of responsibility**. `middle/core/` holds `ir.rs` + `ir_gen.rs` + `bytecode.rs` — three things, the name is already invalid; AST is consumed by all layers, putting it under `frontend/` would mislead downstream paths (industry majority of 6/7 vendors place it as a top-level domain, peer to parser) |
| D2  | Naming conventions                                                                                                                                                         | Keep `type_.rs` / `fn_.rs` (to avoid Rust 2024 reserved words); forbid `xxx_v2` / `xxx_new` directories                                                    | Parallel copies are a common source of dead code (the `Precedence` enum in `pratt/precedence.rs` runs parallel to the actual BP constants in effect, zero production references; `Expr::FnDef` is a never-executed parallel construction path)                                                                                                                                            |

### Stage Contract (02)

| #   | Topic                                                                                                                                                                                                                                 | **Decision**                                                                                                                                               | Rationale                                                                                                                                                                                                |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D3  | Whether to merge `check_module` (`typecheck/mod.rs:81`, fail-fast; early-return at `inference/statements.rs:2797-2799`) vs `check_module_collect_all` (`typecheck/mod.rs:93`, collect-all; call sites `orchestrator.rs:124` / `:493`) | **Do not merge.** Keep both APIs, driven by the aggregation-mode field of `Program` (`Aggregation: FailFast \| CollectAll`, see 02)                        | The two have genuinely different semantics (`CollectAll` serves LSP). Forcing a merge would introduce `Option` noise                                                                                     |
| D4  | Which of the 16 fields of `Obligations` "must be consumed"                                                                                                                                                                            | **All 16 enter the ledger.** Fields with no consumer (e.g., `module_namespaces`) are removed in P4's S4, **no orphan fields left behind**                  | That is the whole point of the ledger — no exceptions                                                                                                                                                    |
| D5  | Diagnostic level of `assert_drained()`                                                                                                                                                                                                | **Final E level** (error); within P4, first go live as W for observation, then promote to E at the end of the same phase (the two-step approach in 02 §S4) | Stalling at W long-term equals having no gate — W does not change exit code; but a one-step jump to E makes the C2 oracle unusable, so E is the final state and W is only a transitional state within P4 |
| D6  | `ownership.rs:627` creating a new Z3Backend each time                                                                                                                                                                                 | **Change to a singleton**, unified with the global `LazyLock` at `predicate.rs:34-36` into `SolverProvider`                                                | Three acquisition strategies and two failure philosophies (one panics, one is silent) must converge                                                                                                      |
| D7  | `layers/README.md` layer order is the reverse of actual                                                                                                                                                                               | **Fix**, assigned to P4's 4.4.1                                                                                                                            | The README describes an intent-architecture that was never built; the fix will expose the previously-silenced `Unproven`, **which is supposed to be exposed**                                            |

### Type Representation (03)

| #   | Topic                                                                                           | **Decision**                                                                                                                                                                                            | Rationale                                                                                                                                     |
| --- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| D8  | Whether to delete `AssocType` (`ast.rs:463-471`)                                                | **Delete.** Zero production construction (the sole construction in the whole repo is in test `types/tests/mono.rs:178`, which does not change the conclusion); independently re-verified by the T1 gate | Same handling as the other 11 zero-construction variants; if associated-type syntax is enabled in the future, a new proposal will redefine it |
| D9  | T1 gate uses `syn` or Python                                                                    | **Use `syn`.** `tools/code-tables` is already a Rust crate, reuse it                                                                                                                                    | Building another Python parser is duplicating implementation, violating prohibition one                                                       |
| D10 | What does `Type::Void` fallback (`parser/statements/types.rs:836`) become                       | **Change to `Err(Diagnostic)` return**                                                                                                                                                                  | Silent fallback is a form of "invention" — using a fake value to mask absence                                                                 |
| D11 | `NameKind` determination + LSP path probe                                                       | **LSP reuses `TypeEnvProbe`**, do not open a separate probe channel                                                                                                                                     | `lsp/world.rs` already holds `SemanticDB`, reuse cost is low                                                                                  |
| D12 | `const_data::BinOp` / `ast::BinOp` who generates whom                                           | **Use `ast::BinOp` as the source**, the `const_data` side aligns semantically (do not forcibly rename `Neq`/`Ne`)                                                                                       | AST is the main representation; renaming ripples to 12 sites with no payoff                                                                   |
| D13 | `classify_generic_params` uses `signature_params` or `Lambda.params`; same for `ir_gen.rs:1380` | **Not an open item, it is the mandatory verification item at P6 stage 6.1.3**, conclusion goes in the PR                                                                                                | This is an execution step, not a pending strategy                                                                                             |
| D14 | Statistics of non-canonical type names in the 293-corpus                                        | **Same as above, it is a task** (6.1.1), not an open item                                                                                                                                               | Same                                                                                                                                          |
| D15 | Classification of "no behavior change" vs "behavior change" in stage 2                          | **Classify by C3 oracle: design changes go in an independent commit, wording changes do not count**                                                                                                     | Already defined in 07                                                                                                                         |

### Intermediate Representation (04)

| #   | Topic                                                           | **Decision**                                                                                                                                                                                       | Rationale                                                                                                                                                                                                                                                                                   |
| --- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D16 | Static selection of `Instruction::Phi`                          | **Plan A: IR-only, `codegen` expands into a Move sequence**                                                                                                                                        | No new `PHI` opcode, avoid expanding the `.42` format again                                                                                                                                                                                                                                 |
| D17 | `bytecode.rs:2312` `upvalue_count: 0`                           | **Fix; when adding the field, bump the `.42` version from 4 to 5** (the format header already has `MAGIC` + `VERSION: u32` field, `codegen/bytecode.rs:14-16`, the read side validates by version) | This is a data-loss defect, not dead code; after the version bump, old `.42` is rejected by the read side under existing behavior (`.42` is a build artifact, cross-version compatibility is not a goal); already-allocated-but-unused opcode values are handled alongside the version bump |
| D18 | After deletion, whether the u8/255 slot cap is sufficient       | **Must be measured at the end of batch d**; if insufficient, expand to `u16`                                                                                                                       | This is a verification item, not a pending item                                                                                                                                                                                                                                             |
| D19 | Should the linear-scan allocator be prototyped for timing first | **No prototype, implement directly.** Wall time is measured in CI                                                                                                                                  | Timing does not change the design decision, it is procrastination                                                                                                                                                                                                                           |
| D20 | Span keying for `ReleasePlan` / `overload_resolutions`          | **Change to `PlanId`**, eliminate cross-layer span contract; assigned to P4                                                                                                                        | Span mismatch causes silent Drop loss, the most hidden class of defect                                                                                                                                                                                                                      |
| D21 | Does `method_def_ordinals` belong in 03 or 04                   | **Belongs in 04** (P7 batch c)                                                                                                                                                                     | It is IR-construction-time state, not a type issue                                                                                                                                                                                                                                          |
| D22 | Whether the `synth.rs` boundary is generalized to all of L3     | **Generalize to all of L3**; L4 is checked separately and brought in                                                                                                                               | Boundary rules are either complete or invalid                                                                                                                                                                                                                                               |
| D23 | Whether `compile_pattern` / `eval_const_expr` "does not handle" | **Brought into batch d scope**, not exempt                                                                                                                                                         | Using "marked as not handled" to evade work is patch-thinking                                                                                                                                                                                                                               |

### Frontend (05)

| #   | Topic                                                                          | **Decision**                                                                                                                                     | Rationale                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D24 | Can LALRPOP fully reproduce existing diagnostics                               | **Must reproduce.** C5 is not relaxed. Use explicit error productions to write `synchronize()`'s sync-point set and skip timing into the grammar | Failure to do so is an implementation defect, **not a reason to relax the oracle**; report honestly and re-evaluate (including "keep Pratt"), do not change the oracle |
| D25 | The f-string span being determined at lex time will change, is this acceptable | **Not acceptable.** The span must be recorded at lex time with **absolute offset**, the diff compares `span.file/line`                           | Span changes affect diagnostic location, not acceptable                                                                                                                |
| D26 | Does associativity need explicit declaration                                   | **Needed.** The grammar explicitly declares left/right associativity, not the `bp_right = bp_left + 1` convention                                | The current convention is exactly the source of magic numbers like `(6,7)` hard-coded in `BP_RANGE`                                                                    |
| D27 | Handling of `is_old_function_syntax` (36 lines)                                | **Option 1: deleted naturally as the grammar migrates.** `f(Int) -> Int = ...` cannot match any production                                       | Specifically probing removed syntax is an anti-pattern                                                                                                                 |
| D28 | Does stage 0 delete `pub use precedence::*;` in `pratt/mod.rs:11`              | **Delete**                                                                                                                                       | It is the exposure surface of the 96-line dead staircase                                                                                                               |
| D29 | The 8-segment responsibility of `parse_assign_after_target`                    | **All split, none retained.** `apply_semantic_side_effects` is handed off to P6 but **not deleted**                                              | Splitting is responsibility re-division, not responsibility deletion                                                                                                   |
| D30 | Full-corpus time of stages 2-4                                                 | **Measure and record in the P2 baseline stage**                                                                                                  | It is a verification item, not a pending item                                                                                                                          |

### Cleanup and Oracle (06 / 07 / 08)

| #   | Topic                                                           | **Decision**                                                                                                                                                                                                                                                                                          | Rationale                                                                                                                   |
| --- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| D31 | Which PR fixes the defects exposed by S1's revived tests        | **The same PR**, test revival and defect fix together                                                                                                                                                                                                                                                 | Splitting them up tempts future "fix it next time"                                                                          |
| D32 | `Switch` opcode (interpreter has implementation, no producer)   | **Delete the opcode and the interpreter implementation.** Handled alongside the `.42` version bump (D17)                                                                                                                                                                                              | An opcode with no producer is a dead path, the whitelist is just cover                                                      |
| D33 | `UnaryOp::Not` silently degrades to `I64_NEG`                   | **Fix.** `opcode()` must distinguish the `op` field, not ignore it                                                                                                                                                                                                                                    | Silently changing semantics is the most dangerous class                                                                     |
| D34 | Encoding branch of `Instruction::TailCall`                      | **Delete** (no construction site in the whole repo)                                                                                                                                                                                                                                                   | Same reasoning as D32                                                                                                       |
| D35 | The cfg branch in 27 wasm files                                 | **Delete the unreachable parts in the playground scenario, keep the reachable ones.** Assigned to P10                                                                                                                                                                                                 | The wasm target is already built (an independent shim crate), the branch is load-bearing, only delete the unreachable parts |
| D36 | Does `TRACKING.md` add an "Implementation Status" column        | **Add.** `check_tracking.py` gains generation logic                                                                                                                                                                                                                                                   | All 26 accepted RFCs currently do not record implementation status — that is how RFC-018 rotted                             |
| D37 | Dead references in `docs/superpowers/` § (gitignored documents) | **Delete the comment**                                                                                                                                                                                                                                                                                | Pointing to a gitignored planning document is a dead reference                                                              |
| D38 | If `verify_loose` cannot run green                              | **No exemption.** If it cannot run green, it means `ir_gen` has implicit "same-slot-multiple-write" dependencies, **which is a defect that must be fixed first**                                                                                                                                      | An exemption list is new technical debt, it is equivalent to using a gate to mask a design problem                          |
| D39 | Snapshot baseline size and compression                          | **Do not introduce git-lfs, accept the size**                                                                                                                                                                                                                                                         | Snapshots are regression oracles, readability over size                                                                     |
| D40 | Multi-file corpus layer                                         | **Unconditionally required** (P2's 2.3.2), not "P4 is paused if not done"                                                                                                                                                                                                                             | A conditional clause is a back door for oneself                                                                             |
| D41 | Whitelist of `test_release_plan_spans_consumed`                 | **No whitelist, the difference set must be empty** (this test is new in P2; the existing `WHITELIST = ["SWITCH"]` in the repo belongs to the opcode round-trip test and is unrelated to this item). If it cannot be empty, it means the ReleasePlan contract has a defect, fix the contract (see D20) | A whitelist is legalizing the bug                                                                                           |
| D42 | Does `verify()` hook into `cargo test` or CI                    | **CI**                                                                                                                                                                                                                                                                                                | Full-corpus verification time is not suitable for every local run                                                           |
| D43 | Similarity algorithm for prohibition one A oracle               | **Jaccard ≥ 0.5**                                                                                                                                                                                                                                                                                     | Already fixed in the body of `08`, delete the open item                                                                     |
| D44 | Does PR require a responsibility attribution declaration        | **Yes.** Add a required field to the PR template                                                                                                                                                                                                                                                      | Responsibility determination is not machine-checkable, can only be enforced by process                                      |
| D45 | Who reviews `// reason:` exemptions                             | **Folded into the PR review checklist**, no dedicated person                                                                                                                                                                                                                                          | With no team division, designating a dedicated person is the same as having none                                            |

### Project and Process

| #   | Topic                                                              | **Decision**                                                                                                                                                                                                             | Rationale                                                                                                                                                                                                                                                                                                           |
| --- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D46 | `RFC-018` (accepted, 0 lines of code)                              | **Move back to `draft/`**                                                                                                                                                                                                | Per `rfc/index.md:131`'s definition, "accepted = enters implementation phase" — it has not                                                                                                                                                                                                                          |
| D47 | Phase-to-Issue mapping                                             | **Open one Issue per first-level phase**, register via `check_tracking.py`'s `issues_impl`                                                                                                                               |                                                                                                                                                                                                                                                                                                                     |
| D48 | Multi-file corpus layer location                                   | **`tests/yaoxiang-multifile/`** (new)                                                                                                                                                                                    | Extending `multifile.rs` would mix semantic tests and contract corpus                                                                                                                                                                                                                                               |
| D49 | Doc site navigation                                                | **Add.** A new section is added next to "Tooling Design" in `config.js:255`                                                                                                                                              | Without it, the nine companion documents are unreachable                                                                                                                                                                                                                                                            |
| D50 | Phase parallelization (multi-file typecheck)                       | **Not in this round.** Becomes an independent topic after the oracle stabilizes                                                                                                                                          | Parallelization would mask order-dependency defects, conflicting with this round's investigation goal                                                                                                                                                                                                               |
| D51 | P3 hemostasis channel                                              | **Allowed.** P3's prerequisites narrow to 2.3.2 + 2.4.1 + 2.4.3; the rest of P2 (IR verifier / snapshots / single-file diff / performance baseline) can run in parallel with P3; P4 must wait for P2 to complete in full | The hemostasis of the correctness vulnerability (silent pass of `Sorted(3)`) should not be blocked by snapshot-infrastructure build-out; P4 needs all three oracle layers in place, so it is not relaxed                                                                                                            |
| D52 | Three `.42` hard-coded discards (`bytecode.rs:2312`/`2315`/`2341`) | **All rolled into P7 (7e), no independent issue left.** `2315` (exception table) and `2341` (globals) were originally planned as independent issues, now consolidated                                                    | The exception table loss makes throw/try behave incorrectly when `.42` is run directly — throw/try are core language semantics; this refactor leaves no to-be-implemented leftovers for core functionality. Fixing all three together requires only one `VERSION` bump (4→5), cheaper than splitting into two bumps |
| D51 | P3 hemostasis channel                                              | **Allowed.** P3's prerequisites narrow to 2.3.2 + 2.4.1 + 2.4.3; the rest of P2 (IR verifier / snapshots / single-file diff / performance baseline) can run in parallel with P3; P4 must wait for P2 to complete in full | The hemostasis of the correctness vulnerability (silent pass of `Sorted(3)`) should not be blocked by snapshot-infrastructure build-out; P4 needs all three oracle layers in place, so it is not relaxed                                                                                                            |
| D52 | Three `.42` hard-coded discards (`bytecode.rs:2312`/`2315`/`2341`) | **All rolled into P7 (7e), no independent issue left.** `2315` (exception table) and `2341` (globals) were originally planned as independent issues, now consolidated                                                    | The exception table loss makes throw/try behave incorrectly when `.42` is run directly — throw/try are core language semantics; this refactor leaves no to-be-implemented leftovers for core functionality. Fixing all three together requires only one `VERSION` bump (4→5), cheaper than splitting into two bumps |

### Sole Outstanding Data Item

| #   | Item                | Description                                                                                                                                                                              |
| --- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| —   | RFC's `issue` field | Currently `TBD`. **A real Issue number must be filled in before opening a PR**, otherwise while `check_tracking.py` can pass, `scripts/rfc/ai_agent.py`'s `issue_rfc_link` will not link |

## Appendix: Glossary

| Term                                       | Definition                                                                                                                         |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Stage**                                  | A single exhaustible step in a compilation. The stage set must be an exhaustible enum, not runtime-registered                      |
| **Orchestration layer (L1)**               | The layer that decides which stages to run, how many times, and how failures propagate                                             |
| **Frontend layer (L2)**                    | Lexing and syntax, producing the AST                                                                                               |
| **Intermediate representation layer (L3)** | Type checking, static analysis, IR construction                                                                                    |
| **Execution layer (L4)**                   | Bytecode generation, interpreter, runtime, standard library                                                                        |
| **Obligation**                             | A contract item produced by some stage that must be consumed downstream, otherwise the compilation fails                           |
| **Obligations ledger (Obligations)**       | The container of all obligations, settled uniformly at the end of the stage table                                                  |
| **Orphan**                                 | A file that is never referenced by any `mod` declaration, and therefore never participates in compilation                          |
| **Shell**                                  | A file that has been declared, enters the compilation product, but only contains doc comments or unused `use` with zero assertions |
| **Equivalence oracle**                     | An executable check that proves behavior is unchanged before and after a refactor                                                  |
| **C1–C6**                                  | Refactor-category grading, determines oracle strength. **There is no C5′**                                                         |
| **Responsibility separation**              | The only solution to scale problems: one module carries one class of responsibility. **No line-count / volume gate**               |
| **Decision registry**                      | The "Decision Registry" section of this RFC, the final ruling on all open items                                                    |

## Appendix: Design Decision Log

| Decision                    | Resolution                                                                                                                                   | Date       | Recorder  |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------- |
| Layering granularity        | Four layers (L1–L4)                                                                                                                          | 2026-10-03 | ChenXu233 |
| Document structure          | **1 RFC + 9 companion design documents** (`docs/src/dev/architecture/`, peer to `check/`, `formatter/`)                                      | 2026-10-03 | ChenXu233 |
| Directory rename            | **Done** (D1), not staged                                                                                                                    | 2026-10-03 | ChenXu233 |
| AST location                | **Top-level `src/ast/`**, peer to parser (industry majority; check-boundary exemption clause)                                                | 2026-10-04 | ChenXu233 |
| Opcode location             | **`middle/bytecode/opcode.rs`**, merged with the bytecode domain (table and format share the domain, dependency direction restored to L4→L3) | 2026-10-04 | ChenXu233 |
| Size gates                  | **All cancelled.** Size is solved by responsibility separation, adopting the Go official position                                            | 2026-10-03 | ChenXu233 |
| Syntax paradigm             | **Full LALRPOP grammar-driven**; reject the "keep Pratt but tabulate" middle state                                                           | 2026-10-03 | ChenXu233 |
| Equivalence oracle          | **C1–C6 six-tier grading, no relaxation** (there is no C5′)                                                                                  | 2026-10-03 | ChenXu233 |
| Deletion strategy           | Full deletion is allowed; orphan tests with real assertions are **revived, not deleted**                                                     | 2026-10-03 | ChenXu233 |
| Verification method         | This document set does not run cargo; static evidence + line numbers are the source of truth                                                 | 2026-10-03 | ChenXu233 |
| Phase order                 | P0 maintenance mechanism → P1 revive tests → P2 oracle → … → P10 cleanup                                                                     | 2026-10-03 | ChenXu233 |
| Open items                  | **All 50 nailed down** (Decision Registry D1–D50), no trade-offs left                                                                        | 2026-10-03 | ChenXu233 |
| P3 hemostasis channel       | **Allowed** (D51), prerequisites narrow to 2.3.2+2.4.1+2.4.3                                                                                 | 2026-10-05 | ChenXu233 |
| Three `.42` data-loss sites | **All rolled into P7 (7e), no independent issue left** (D52)                                                                                 | 2026-10-05 | ChenXu233 |

## References

- [RFC-010 Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Direct source of
  type-representation convergence
- [RFC-011 Generic Type System](../accepted/011-generic-type-system.md)
- [RFC-011a Interface Implementation](../accepted/011a-interface-implementation.md)
- [RFC-013 Error Code Specification](../accepted/013-error-code-specification.md) — The
  generation-time gate example
- [RFC-027 Compile-Time Evaluation and Types](../accepted/027-compile-time-evaluation-types.md)
- [RFC-029 Module Semantics](../accepted/029-module-semantics.md)
- [RFC-036 Test Framework](../accepted/036-test-framework.md)
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](029a-module-cache-incremental.md)
- `src/frontend/core/typecheck/layers/README.md` — Current layer-order declaration (the reverse of
  actual, D7 requires fix)
- `build.rs:19-55` — Existing `panic!`-level gate example in this repo
- `docs/src/dev/design/check/` — Three documents describing an architecture that never existed (see
  `06-cleanup-inventory.md` §C)
