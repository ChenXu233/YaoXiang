---
title: 'RFC-039: Compiler Architecture Refactor (Master Plan)'
status: 'Accepted'
author: 'ChenXu233'
created: '2026-10-03'
updated: '2026-10-05'
accepted: '2026-10-05'
issue: '#430'
---

# RFC-039: Compiler Architecture Refactor

## Summary

This RFC proposes an architecture refactor for the YaoXiang compiler (`src/` 160,002 lines / 509
`.rs` files): re-establishing boundaries through a **four-layer model** (Orchestration / Frontend /
Intermediate Representation / Execution), establishing an **exhaustible compilation stage contract**
and an **obligations ledger**, together with a set of **equivalence oracles** and
**regression-prevention gates**.

The detailed design is placed in nine companion documents under `docs/src/dev/architecture/`. This
RFC is only responsible for: the problem, root cause, four-layer model, acceptance criteria grading,
and execution stage ordering. **Reviewing this RFC is sufficient to decide whether to approve the
project; the companion documents are construction blueprints after approval.**

## Motivation

### One: Boundaries Exist, but Have No Enforceability

The directory-level layering (`frontend` / `middle` / `backends`) is clear. But **boundaries exist
only in people's self-awareness, with no mechanism to prevent erosion**. Three direct consequences.

#### 1.1 Compilation Stages Are Manually Wired at Five Entry Points, Producing 11 Behavioral Inconsistencies

| Stage                      | Single-file `run` | Multi-file `run` | `check` | LSP (in-project) | LSP (single-file) |
| -------------------------- | ----------------- | ---------------- | ------- | ---------------- | ----------------- |
| lexing / parsing           | Yes               | Yes              | Yes     | Yes              | Yes               |
| typecheck                  | Yes               | Yes              | Yes     | Yes              | Yes               |
| **proof_execution**        | **Yes**           | **No**           | **No**  | **No**           | **No**            |
| dead-code analysis         | Yes               | No               | Yes     | No               | No                |
| W1006 shadowing diagnostic | No                | No               | Yes     | No               | No                |
| monomorphization           | Yes               | No               | No      | No               | No                |
| IR generation / linking    | Yes               | Yes              | No      | No               | No                |

The five entry points: `src/frontend/pipeline.rs:141-227` (single-file, 5 stages at `149` / `161` /
`173` / `188` / `205`), `src/frontend/compiler.rs:106` (wrapper), four in
`src/frontend/module/orchestrator.rs` (`compile_project:99` / `check_project:273` /
`check_source_in_project:450` / `compile_embedded_module:1374`), and `src/lib.rs` `run_file:140` /
`run_project:154`.

The most severe cell causes a **correctness bug**. Proof function constraints of the form
`x: Sorted(3)` **silently pass** under the multi-file, check, and LSP paths. Evidence chain (all
reproducible):

1. `src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** location in the entire
   repository that constructs a non-empty `proof_calls`, with the semantics "this constraint
   requires executing a proof function".
2. `src/frontend/core/typecheck/checker.rs` has three isomorphic branches handling `Unproven`:
   `5164` / `5306` / `5420` are `if calls.is_empty()` → hard error; `5179` / `5318` / `5448` are
   `ctx.proof_calls.extend(...)` → **no diagnostic produced**. The comment at `5165` literally
   reads: "Unproven → compile error, no degradation, no silent pass……do not let silent pass be
   revived".
3. `TypeCheckResult.proof_calls` (`types.rs:29`) is read in the **only one location** in the entire
   repository: `src/frontend/pipeline.rs:187`.
4. The four entry points in `orchestrator.rs` **all do not read this field**.

That is: the invariant declared by the code itself is broken by architecture rather than logic. The
existence of `compile_embedded_module` (`orchestrator.rs:1374`) means **the refinement obligation of
the embedded std module itself also takes this discarding path**.

**No test can catch it**: the 27 tests in `tests/integration/multifile.rs` have zero hits on
`Sorted` / `proof` / `refin`; the unit tests of RFC-027 directly call `check_module` to assert
"`proof_calls` is non-empty", **stopping right before pipeline** — they verify "it was populated",
while the bug is "the consumer does not read it".

#### 1.2 Module Boundaries Can Be Bypassed Without Trace

`src/frontend/core/typecheck/checker.rs:5618` is `include!("checker/semantic_tokens.rs");` — **the
only `include!` in the entire repository**. It splices 1547 lines of text into the `checker` module.
This file **is not declared as any module** (`grep 'mod semantic_tokens'` returns zero hits in the
entire repo), its first line is directly `impl TypeChecker {`, without its own `use` header, and
`checker/` directory has only this one file.

Consequences: no module identity, visibility isolation fails, rust-analyzer's jump and symbol search
fail on it, and the toolchain counts it as part of `checker.rs` when analyzing. **The real size of
the `checker` module is 5618 + 1547 ≈ 7165 lines**, not the 5618 lines shown by directory
statistics.

#### 1.3 Test Wiring Relies on Human Memory, Corruption Happens Silently

Performing BFS reachability analysis of the module graph from the two crate roots `src/lib.rs` /
`src/main.rs`: **24 files / 2628 lines have never participated in compilation**. Among them, 23
files / 1081 lines are real orphans.

| Orphan                                      | Scale                | Content                                                                              |
| ------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------ |
| `frontend/core/lexer/tests/` entire subtree | 13 files / 686 lines | 7 empty shells totaling 19 lines + `mod.rs` 38 lines + **629 lines / 55 real tests** |
| `frontend/pipeline/tests/`                  | 3 files / 14 lines   | All placeholder documentation comments                                               |
| `package/template/tests/`                   | 3 files / 65 lines   | 7 `#[test]`                                                                          |
| `parser/pratt/tests/precedence_inline.rs`   | 95 lines / 6 tests   | Hidden in a live directory                                                           |
| `typecheck/passes/tests/overload_inline.rs` | 170 lines / 7 tests  | Same as above                                                                        |
| `util/diagnostic/emitter/tests/json.rs`     | 46 lines / 3 tests   | Same as above                                                                        |

`cargo test` cannot discover them: tests that have never run do not fail. CI
(`.github/workflows/ci.yml:140`) also has no test count baseline or wiring check.

**Real tests that have never run total 1005 lines / 78 tests.** The lexical layer actually running
has only `fstring.rs` (spliced in via `#[path]` attribute bypass).

### Two: Design Capability Is Sufficient

It must be acknowledged simultaneously: this project **already has** mature enforcement mechanisms,
they just have not been generalized.

The error codes in `src/util/diagnostic/codes/` (137 E codes + 8 W codes = 145) are guarded hard at
build time by `build.rs:19-55` through `tools/code-tables` — parse the registry, validate uniqueness
and section positions, compare item-by-item against the RFC-013 markdown code table, **any
inconsistency directly `panic!` to refuse compilation**. RFC-013 documentation and code are
therefore always consistent.

`src/package/` (76 files / 13,012 lines) is the most complete region: **tests 6,227 lines, 47.9%**;
5 of the 6 `tests/` directories are correctly wired (the only exception is `template/tests/`, see
section A). Note that this positive sample holds at the **directory-level wiring**, not at every
file — among 76 `.rs` files, only 7 contain inline `#[cfg(test)]` and 5 contain `mod tests;`. The 5
RFCs match the code item by item, and many "not implemented" items fall to explicit errors rather
than silent TODOs (cmake in `build/mod.rs:172-174`, `RegistryDeferred` in `error.rs:74-76`).
`.yxpkg` has five layers of safety protection and all are correct.

**The same repository, the same authors: `package/` did the test directory wiring right, while
`lexer/tests/` has never been run once. The difference is not in capability, it is whether someone
runs a check.**

### Three: Phantom Designs

The three documents under `docs/src/dev/design/check/` describe an architecture that **has never
existed**: `CheckSession` (including the Rust code draft in `incremental-checking.md:31-41`),
`ModuleDependencyGraph`, `ModuleCache`, `HotReloader` — all zero hits in the repository; the
`traits/` directory does not exist; the `check_single_module` function does not exist (only lives in
test comments); even the `command.rs` busy-wait referenced in "known limitations" does not exist
(that file has neither `Instant` nor `recv_timeout`).

Cross-file analysis is actually implemented by `src/frontend/module/` (registry 439 + resolver 195 +
roles 388 + orchestrator 1528, about 2670 lines) — the capability is real, the path is completely
different. **This document writes "designs that might exist in the future" as "defects of an already
implemented system".**

`src/frontend/pipeline/tests/compilation_cache.rs` and `incremental_scheduler.rs` (3 lines each) are
the cleanest physical evidence of "test skeletons built per design, found no corresponding
implementation in the pipeline, and were shelved".

Additionally, `RFC-018` (LLVM AOT, 1037 lines, status accepted) has 0 lines of implementation in the
code — all 6 AOT-related hits in `backends/mod.rs` are **comments**.

## Root Cause Diagnosis

Combining the three symptoms, the root cause is singular:

> **This project treats "design" as a documentation convention, not as an enforceable constraint.**

| Layer            | Missing Enforcement                                                  | Existing but Ungeneralized Example                                         |
| ---------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Stage boundary   | A single decision point for "which stages this compilation runs"     | —                                                                          |
| Module boundary  | Prohibiting `include!`, restricting `pub(crate)` cross-layer leakage | —                                                                          |
| Cross-references | Detectability of "field produced but unconsumed"                     | `build.rs` error code gate                                                 |
| Test wiring      | Declaring a directory must have a `mod` declaration                  | `src/package/` directory-level wiring convention (5 of 6 `tests/` correct) |

This RFC's position is therefore: **transform the three types of boundaries — stages, modules, and
cross-references — from "documentation conventions" into "facts assertable at compile time or test
time"**, and **establish a single stage decision point at the orchestration layer**.

## Proposal

### Four-Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│ L1 Orchestration                                            │
│   Who decides which stages run, how many times,            │
│   and how failures propagate                                │
│   Current: pipeline.rs(740) + orchestrator.rs(1528)         │
│         + compiler.rs(267) = 5 entry points, 11 inconsistencies │
│   Goal: Single Driver + exhaustible Stage enum + Obligations ledger │
│   See: ../compiler-architecture/02-stage-contract.md           │
└───────────────────────────┬─────────────────────────────────┘
                            │ Compilation Unit (Unit)
┌───────────────────────────▼─────────────────────────────────┐
│ L2 Frontend                                                 │
│   Lexical, syntactic, AST construction                      │
│   Current: lexer/(3065) + parser/(6341)                    │
│         Hand-written lexer + hand-written Pratt;            │
│         adding an operator requires changes in 6-7         │
│         places in L2 + 17 downstream production files      │
│   Goal: Lexical converges to single implementation +        │
│         LALRPOP grammar-driven syntax                        │
│   See: ../compiler-architecture/05-frontend-paradigm.md        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Module (AST)
┌───────────────────────────▼─────────────────────────────────┐
│ L3 Semantic & IR                                            │
│   Type checking, static analysis, IR construction           │
│   Current: typecheck/(29466 production) + ir.rs(905)       │
│         + ir_gen.rs(8448)                                   │
│         3 parallel type representations + 3 parallel        │
│         operator enums;                                     │
│         ir_gen single impl 7952 lines / 116 methods;        │
│         no IR-level validator                               │
│   Goal: Type representation singularized;                   │
│         IR satisfies SSA construction discipline            │
│   See: ../compiler-architecture/03-type-unification.md         │
│         ../compiler-architecture/04-ssa.md                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ ModuleIR
┌───────────────────────────▼─────────────────────────────────┐
│ L4 Execution                                                │
│   Bytecode generation, interpreter, runtime, std lib        │
│   Current: codegen/(3436) + bytecode.rs(2422)               │
│         + executor/(2000) + runtime/(1800) + std/(7543)     │
│   Goal: opcode single source of truth + generation-time gate;│
│         L4 is unaware of L1-L3                               │
│   See: ../compiler-architecture/06-cleanup-inventory.md        │
└─────────────────────────────────────────────────────────────┘
```

**The four layers are strictly unidirectional**: L1 depends on the interfaces of L2/L3/L4; L2 does
not depend on L3; L3 does not depend on L2 (only consumes AST data, does not call parser); L4 only
depends on L3's output format.

The list of known reverse dependencies and parallel definitions is in routing table C of
`../compiler-architecture/01-routing.md`.

### Companion Design Documents

Nine documents, stored in `docs/src/dev/architecture/`:

| Document                          | Layer            | Topic                                                                                                                                                                                                                                          |
| --------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `01-routing.md`                   | Cross-layer      | **Function routing table**, dependency direction specification, **target directory structure after construction completion**, future extension guidelines                                                                                      |
| `02-stage-contract.md`            | L1               | Stage contract and obligations ledger: 11 inconsistencies, vulnerability evidence chain, `Stage` / `Obligations` / `Driver` / `ProgramKind` (including `WasmPlayground` on the wasm playground side)                                           |
| `03-type-unification.md`          | L2/L3 root cause | Type representation singularization: disposition of 26 `ast::Type` variants item by item, 13 forward-zero-construction variants, type table gate                                                                                               |
| `04-ssa.md`                       | L3               | Intermediate representation SSA-ization: four types of defects, SSA form definition, allocator evaluation, 38 change items                                                                                                                     |
| `05-frontend-paradigm.md`         | L2               | Lexical converges to single implementation / syntax LALRPOP grammar-driven, change surface convergence, dead step handling, test reconstruction                                                                                                |
| `06-cleanup-inventory.md`         | Global           | Dead code and phantom design cleanup: reachability method, item-by-item disposition, execution order                                                                                                                                           |
| `07-equivalence-oracle.md`        | Cross-layer      | Equivalence oracle: three layers of criteria, C1-C6 grading, gate design                                                                                                                                                                       |
| **`08-maintenance-mechanism.md`** | **Cross-layer**  | **Repository maintenance mechanism**: three prohibitions (no fabrication / no endless filling / patching when refactoring is needed), D0–D4 decision procedures, machine-checkable rules, code review checklist, external convention reference |
| **`09-execution-wbs.md`**         | **Cross-volume** | **Multi-level construction task table**: 11 primary / 45 secondary / 122 tertiary tasks, dependencies and parallel grouping, **8 conflict registrations**                                                                                      |

`01` and `09` are **long-term reference documents** — the former does not become invalid with any
refactor, the latter is the construction checklist. The three prohibitions produced by `08`
constrain every action of P1–P10.

### Equivalence Oracle Grading

**This is the safety net for the entire refactor, and must be established before any code change.**
See `07-equivalence-oracle.md` for detailed design.

Core principle: **criteria are graded by refactor category, because different categories require
different strengths of equivalence.** Using a single criterion (usually "IR snapshot identical") to
cover all refactors is wrong — some refactors **will inevitably change IR form**.

| Category | Refactor Content                                            | Criterion Type                                         | Strength  |
| -------- | ----------------------------------------------------------- | ------------------------------------------------------ | --------- |
| **C1**   | Pure move (split file, change directory, extract submodule) | IR canonicalized snapshot **zero-diff**                | Strongest |
| **C2**   | Orchestration change (staging, unified Driver)              | Same diagnostic set per entry + same corpus behavior   | Strong    |
| **C3**   | Type representation convergence                             | Same diagnostic **code** (message wording will change) | Strong    |
| **C4**   | IR form change (SSA-ization)                                | **Behavior equivalence** + IR structure invariants     | Medium    |
| **C5**   | Frontend paradigm change                                    | AST snapshot + diagnostic + behavior, all three        | Strong    |
| **C6**   | Pure deletion                                               | No equivalence needed, just confirm no references      | —         |

**The distinction between C1 and C4 is key**: C1 requires IR byte-by-byte identical; C4 acknowledges
IR will change, switching to "same program behavior + IR satisfies invariants". Forcing snapshots on
C4 will induce the team to relax the criteria.

Three layers of criteria:

1. **IR static validator** — dominating use-before-def, jump targets exist, type consistent, inner
   layer isolation. Far stronger than snapshots, covering "IR self-consistent but wrong value"
   defects that snapshot tests cannot detect. Must **first run green on existing (non-SSA) IR**.
2. **Canonicalized IR snapshot** — strip `Span`, rename temporaries in order of appearance,
   relativize global slots, sort predecessors. Commit, manually review diff. **Known limitation**:
   canonicalization will erase "which register the Nth argument used", so **alone insufficient to
   cover `arg_regs` semantic rearrangement**.
3. **End-to-end corpus diff** — diagnostic list, exit code, stdout/stderr compared item by item.

> **⚠️ Structural limitation of corpus coverage**: the `tests/` directory has **no `yaoxiang.toml`**
> (measured 0), therefore the **293** `.yx` corpora in `tests/yaoxiang/` **all take the single-file
> path** (standalone branch of `check_files_with_diagnostics` → `check_single_file` →
> `Pipeline::run`).
>
> **Consequence**: the third layer of criteria **can only validate single-file path changes**. For
> the multi-file side (the four entry points of `orchestrator`) and the multi-file performance of
> types/SSA, **there is currently no corpus coverage**. P4 "Unified Driver" is the biggest risk
> point of the whole plan, **the multi-file corpus layer must be unconditionally established in P2**
> (decision D40) — it is the sole executable behavior criterion source for P4.

## Implementation Strategy

### Stage Sequence

```
P0  Repository maintenance mechanism and code placement procedures 08        ← Set rules first: three prohibitions + D0-D4 decision procedure
P1  Revive orphan tests and fix defects        06 §S1   ← Cheapest, most profitable
P2  Establish equivalence oracle baseline          07 全部
P3  Fix correctness vulnerability (minimal solution)      02 §修复
P4  Stage contract and unified Driver        02
P5  checker file internal split          09 §P5 补齐
P6  Type representation singularization              03
P7  Intermediate representation SSA-ization             04     } parallel
P8  Frontend paradigm change                05     }
P9  Regression-prevention gates (unified script list)     01 + 08
P10 Other cleanup and status correction          06 §S2-S6
```

**Core serial constraints (not interchangeable):**

| Constraint   | Reason                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P0 → P1      | The three prohibitions constrain every action of P1 — 1.2.x is adding `mod` declarations to `mod.rs`, which is exactly how the `precedence_inline.rs` life-and-death conflict arose                                                                                                                                                                                                                                |
| P1 → P2      | Reviving tests will change the test count and corpus baseline. Building the baseline first will become invalid after P1                                                                                                                                                                                                                                                                                            |
| P2 → P3      | The vulnerability criteria must **be written red first** to prove the fix is effective. **Narrowing (D51)**: P3's prerequisites are only 2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red criteria); the rest of P2 can run in parallel with P3, but P4 must wait for P2 to complete — the bleeding stoppage of the correctness vulnerability should not be blocked by criterion infrastructure construction |
| P3 → P4      | Fix the bug first then refactor. Reverse order will let the bug be solidified by the stage table as "established behavior"                                                                                                                                                                                                                                                                                         |
| P4 → P5      | P5 starts with `include!` → real `mod` (09 §P5 5.1), the subsequent checker split touches the same file, must be done consecutively after P4                                                                                                                                                                                                                                                                       |
| P5 → P6      | Consecutive changes to the same file must be separated, otherwise regression cannot be bisected                                                                                                                                                                                                                                                                                                                    |
| P6 → P7 / P8 | If type representation does not converge first, SSA's new IR will grow into a third representation                                                                                                                                                                                                                                                                                                                 |
| P7/P8 → P9   | The initial baselines of each gate script (e.g., `pub(crate)` leakage count) take their final values only after P7/P8                                                                                                                                                                                                                                                                                              |
| P7 ∥ P8      | File sets are disjoint, can be parallel                                                                                                                                                                                                                                                                                                                                                                            |

**Why P0 is an independent stage**: the CI script of P9 is **post-hoc check**, it can only stop bad
code already written; "patching when refactoring is needed" is **pre-hoc judgment**, only the D0–D4
decision procedure can stop it. The two cannot replace each other.

### Key Points for Each Stage

| Stage   | Scope                                                                                                                                                                                        | Acceptance                                                                                                                                                                                                                  | Rollback Point                                                                                              |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **P0**  | `CONTRIBUTING.md` procedure section + 3 new gate scripts (`check-concepts` / `check-fanout` / `check-boundary`); **no compiler source code changes, no line count gates**                    | `check-concepts.py` **must report 3 sets of operator enums and 2 sets of parallel type representations (including `ir::Type` alias) without code changes**; intentional over-limit / new 6th entry-style wiring must be red | Pure addition, deleting scripts suffices                                                                    |
| **P1**  | 5 files supplement `mod` declarations; **1005 lines / 78 tests revived**                                                                                                                     | Test count increases; **expected to expose real defects** (`literals.rs` overflow path, `\x`/`\u` illegal escapes never tested)                                                                                             | Per-file revert; **fixed defects should not be rolled back**                                                |
| **P2**  | New `verify.rs`, snapshot baseline, corpus diff framework, **multi-file corpus layer**, performance baseline (criterion smoke benchmark), `scripts/ci/check-*`                               | `verify_loose` runs green; vulnerability criteria **written red**                                                                                                                                                           | Criterion code can be removed in its entirety                                                               |
| **P3**  | `types.rs` (`proof_calls` made private), three consumer points in `orchestrator.rs`                                                                                                          | Vulnerability criterion **turns green**; **intentional removal of fix must turn red again**                                                                                                                                 | Pure behavior fix                                                                                           |
| **P4**  | New `src/driver/`; rewrite 5 entry points; **4.5: `ReleasePlan` / `overload_resolutions` key `Span` → `PlanId` (D20)**                                                                       | Same diagnostic set + same corpus behavior (C2); span-keyed silent failure drops to zero                                                                                                                                    | **Biggest risk point**; keep old entry points, Driver unwired for rollback                                  |
| **P5**  | `include!` → real `mod` (5.1); `checker.rs` split out `refinement` / `annotations`                                                                                                           | **IR snapshot zero-diff** (C1); `statements.rs:16` `pub(crate)` path unchanged                                                                                                                                              | Per-file revert                                                                                             |
| **P6**  | `ast.rs`, `types/mono.rs`, `solver.rs`, `ir.rs:3`, `bytecode.rs:2353-2390`; stage finale executes directory rename (D1: `typecheck/`→`sema/`, `middle/core/`→`middle/ir/`)                   | Same diagnostic **code** (C3); rename batches are pure move (C1 zero-diff)                                                                                                                                                  | Phased commit, variant disposition and synonym table deletion separated; rename occupies commit exclusively |
| **P7**  | `ir.rs`, `ir_gen.rs` (8448 lines), `bytecode.rs`, `translator.rs`; **7e: three `.42` data loss fixes (`upvalue_count` / `exception_handlers` / `globals`) + `VERSION` 4→5 (D17, D52)**       | **Behavior equivalence + `verify_ssa` green** (C4). **Not IR snapshot full equality**; `.42` round-trip test                                                                                                                | Per-batch revert                                                                                            |
| **P8**  | `lexer/*`, `parser/*`                                                                                                                                                                        | AST snapshot + diagnostic + behavior (C5)                                                                                                                                                                                   | Per-file revert                                                                                             |
| **P9**  | New `scripts/ci/check-*.py` total 10 (unique list see [09](../../dev/architecture/09-execution-wbs.md) §P9: 3 with P0, 1 with P1, 3 with P2, 2 with P4, 1 with P7, P9 closes uniformly hard) | **Intentional obligation without consumer must be red**; **intentional orphan test directory must be red**; **intentional `include!` must be red**                                                                          | Delete scripts                                                                                              |
| **P10** | Document disposition, RFC status correction, `pub` items downgraded visibility, opcode decision                                                                                              | `check_tracking.py` passes; no 404 in documentation site                                                                                                                                                                    | Independent PR                                                                                              |

### Definition of Done (DoD) for Each Stage

A stage is complete only if all of the following are satisfied simultaneously:

1. All corresponding "implementation strategy" steps in the owning document are committed
2. The criteria of that category (C1-C6) in `07-equivalence-oracle.md` are **green in CI**
3. `cargo test` all green, and **test count not lower than at the start of the stage** (prevent
   "delete tests for green light")
4. `cargo clippy --all --all-features -- -D warnings` clean
5. `python scripts/rfc/check_tracking.py` exit code 0
6. The stage has an independent revert unit — **if a stage cannot be reverted alone, it is too large
   and needs to be split further**
7. For stages touching the compilation pipeline or execution path (P4 / P6 / P7 / P8), 07's
   performance baseline comparison has no unexplained >10% regression

### Global Acceptance Gates

After all stages are complete, the following must be true:

| #   | Condition                                              | Verification                                                                    |
| --- | ------------------------------------------------------ | ------------------------------------------------------------------------------- |
| G1  | Single decision point for compilation stages           | Answering "which stages ran, which didn't, why" only needs to look at one place |
| G2  | No "produced but unconsumed" fields                    | `check-obligations.py` green                                                    |
| G3  | `include!` count is 0                                  | `grep -rn 'include!' src/`                                                      |
| G4  | No orphan tests                                        | `check-test-wiring.py` green                                                    |
| G5  | No reverse dependencies                                | `check-module-boundary.py` green                                                |
| G6  | Type representation is unique                          | Parallel representation converges; hand-written synonym tables disappear        |
| G7  | Parser contains no type/predicate hardcoding           | Literals like `"Terminates"` are replaced by data flow                          |
| G8  | Change surface for adding a binary operator ≤ 3 places | Manual review + routing table update                                            |
| G9  | `layers/README.md` describes actual layer order        | Documentation consistent with `check_module_impl`                               |
| G10 | `TRACKING.md` has "Implementation Status" column       | 52 RFCs' implementation status is queryable                                     |

## Trade-offs

### Advantages

- **Provides a landing point for all subsequent changes**. "Adding an operator requires changes in
  many places, missed changes unknown" cannot be improved by memory, only by routing table + gate.
- **Single root cause**. The three symptoms (stage inconsistency, `include!`, test wiring breakage)
  all stem from "boundaries are conventions not constraints" and can be solved by the same type of
  mechanism at once.
- **Non-blocking**. This RFC has zero code changes. The four-layer model is descriptive, and after
  adoption, only part of it can still be executed.
- **Criteria first**. P2 is established before any code change, and the grading design avoids the
  common failure of "snapshot too strict → induce relaxation".

### Disadvantages and Risks

- **Long serial chain**. P1 → P2 → P3 → P4 → P5 → P6 → P7/P8 is nearly fully serial, later stages
  must wait for earlier ones to complete.
- **P1 will introduce new bug workload**, and this is the most easily skipped step "because we need
  to fix bugs so we don't do it first" — but skipping it, all subsequent stage acceptance criteria
  are built on false coverage.
- **P4 is the biggest single-point risk**: one change touches 5 entry points.
- **P6 has the widest impact**: touches the common dependencies of parser, formatter, spawn,
  orchestrator.
- **P7's line count will net increase 900-1600 lines, not net decrease**. Splitting introduces
  boilerplate, `verify` 300-600 lines, register allocation 600-1000 lines. **If "code shorter" is
  the success criterion at project establishment, this stage will be judged as failure** — this
  point must be aligned at project establishment.

## Alternative Plans

### A. Only Split Files, Don't Change Paradigm

Split 8448 lines into 10 files of 800 lines each.

**Not adopted.** Splitting does not eliminate any root cause: the 6 hand-written save/restore are
still scattered across 10 files, the `arg_regs` semantic rearrangement of `generate_call_expr_ir`
still requires manual reasoning, "adding an operator changes many places" is still many places.
Splitting improves navigation, not correctness.

### B. Only Add Gates, Don't Change Structure (including line count ratchet)

**Not adopted as the only plan, but it is part of this RFC.** Gates can prevent regression, but
cannot fix the current state — `ir_gen.rs` already has 8448 lines, gates can only freeze it at this
number.

### C. Start Over, Write a New Compiler Skeleton

**Not adopted.** `src/package/` proves this team can do complex systems right. The problem is not
capability. Moreover, the existing 293 `.yx` corpora provide a ready-made equivalence oracle source;
rewriting from scratch would waste this asset.

### D. Use an Existing Compiler Framework (e.g., LLVM as the only backend)

**Not adopted.** The `Executor` trait (`backends/mod.rs:356`) already has an abstraction point
reserved but only one implementer. Introducing LLVM is the scope of `RFC-018` (1037 lines of design,
0 lines of code), and should be **decided independently** — it changes "what to compile to" rather
than "how to organize code".

## Decision Register: All Open Items Settled

> **This section is the sole authority.** The original `- [ ]` open questions and "conflict
> registrations" in each companion document **all converge to this table**, no longer individually
> retained. **No "TBD", no "deferred", no "optional".** Each gives a decision and reason. If during
> implementation a decision's technical premise is found not to hold, **the correct action is to
> return to this table to change the decision and explain the reason**, not to bypass.

### Directory and Naming

| #   | Issue                                                                                                                                                                                           | **Decision**                                                                                                                                              | Reason                                                                                                                                                                                                                                                                                                                                                                          |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Whether directory rename (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`, `parser/ast.rs` migrated to top-level `src/ast/`, opcode vocabulary migrated to `middle/bytecode/`) is worth doing | **Do it.** No staging, no "optional", completed together with P5/P6 (top-level AST domain and bytecode domain merge in the same batch, both C1 pure move) | Boundaries are guaranteed by CI, but **directory names are the first-glance signal of responsibility**. `middle/core/` contains `ir.rs` + `ir_gen.rs` + `bytecode.rs` — three things, the name has already failed; AST is consumed by all layers, placing it under `frontend/` would mislead downstream paths (6/7 industry standards place it at top-level parallel to parser) |
| D2  | Naming convention                                                                                                                                                                               | Keep `type_.rs` / `fn_.rs` (avoid Rust 2024 reserved words); prohibit `xxx_v2` / `xxx_new` directories                                                    | Parallel copies are a common source of dead code (`Precedence` enum in `pratt/precedence.rs` parallels the actually effective BP constants, zero production references; `Expr::FnDef` is a never-executed parallel construction path)                                                                                                                                           |

### Stage Contract (02)

| #   | Issue                                                                                                                                                                                                                                              | **Decision**                                                                                                                        | Reason                                                                                                                                                                                       |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D3  | `check_module` (`typecheck/mod.rs:81`, fail-fast; early-exit implementation in `inference/statements.rs:2797-2799`) vs `check_module_collect_all` (`typecheck/mod.rs:93`, collect all; call sites `orchestrator.rs:124` / `:493`) whether to merge | **Do not merge.** Keep two APIs, driven by `Program`'s aggregation mode field (`Aggregation: FailFast \| CollectAll`, see 02)       | The two semantics are indeed different (`CollectAll` serves LSP). Forcing a merge would introduce `Option` noise                                                                             |
| D4  | Which of the 16 fields of `Obligations` are "must be consumed"                                                                                                                                                                                     | **All 16 enter the ledger.** Fields without consumers (e.g., `module_namespaces`) are deleted in S4 of P4, **no orphan fields**     | The meaning of the ledger is no exceptions                                                                                                                                                   |
| D5  | Diagnostic level of `assert_drained()`                                                                                                                                                                                                             | **Final E level** (error); within P4, first observe online at W level, then upgrade to E at stage end (two-step approach of 02 §S4) | Staying at W long-term equals no gate — W does not change exit code; but one-step upgrade to E makes C2 criteria unusable, so E is the final state, W is only the transition state within P4 |
| D6  | `ownership.rs:627` creates a new Z3Backend every time                                                                                                                                                                                              | **Change to singleton**, unify with the global `LazyLock` of `predicate.rs:34-36` into `SolverProvider`                             | Three acquisition strategies, two failure philosophies (one panic, one silent) must converge                                                                                                 |
| D7  | `layers/README.md` layer order is opposite to actual                                                                                                                                                                                               | **Fix**, attributed to 4.4.1 of P4                                                                                                  | The README describes an unimplemented intended architecture; fixing it will expose the previously silenced `Unproven`, **which should be exposed**                                           |

### Type Representation (03)

| #   | Issue                                                                                       | **Decision**                                                                                                                                                                                                                                                                                                                                                                                                                | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D8  | Whether `AssocType` (`ast.rs:463-471`) is deleted                                           | **Delete.** Zero production construction (the only construction in the entire repo is in test `types/tests/mono.rs:178`, does not change the conclusion), re-verified with T1 gate independently                                                                                                                                                                                                                            | Same disposition as the other 11 zero-construction variants; if associated type syntax is enabled in the future, redefine by a new proposal                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| D9  | T1 gate uses `syn` or Python                                                                | **Use `syn`.** `tools/code-tables` is already a Rust crate, reuse it                                                                                                                                                                                                                                                                                                                                                        | Building another Python parser is duplicate implementation, violates prohibition one                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| D10 | `Type::Void` fallback (`parser/statements/types.rs:836`) changed to what                    | **Change to `Err(Diagnostic)` return**                                                                                                                                                                                                                                                                                                                                                                                      | Silent fallback is a form of "fabrication" — using fake values to cover up absence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| D11 | `NameKind` determination + LSP path probe                                                   | **LSP reuses `TypeEnvProbe`**, no separate probe channel                                                                                                                                                                                                                                                                                                                                                                    | `lsp/world.rs` already holds `SemanticDB`, reuse cost is low                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| D12 | `const_data::BinOp` / `ast::BinOp` who generates who                                        | **Take `ast::BinOp` as canonical**, `const_data` side aligns by semantics (do not force rename `Neq`/`Ne`)                                                                                                                                                                                                                                                                                                                  | AST is the main representation; renaming would affect 12 references with no benefit                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| D13 | `classify_generic_params` uses `signature_params` or `Lambda.params`; `ir_gen.rs:1380` same | **Not an open issue, but a mandatory verification item in 6.1.3 of P6**, conclusion written into PR                                                                                                                                                                                                                                                                                                                         | This is an execution step, not a pending strategy                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| D14 | Non-canonical type name statistics in 293 corpora                                           | **Same as above is task** (6.1.1), not an open issue                                                                                                                                                                                                                                                                                                                                                                        | Same as above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| D15 | Stage 2 "no behavior change" vs "behavior change" classification                            | **Classify by C3 criteria: design changes go through independent commit, wording changes not counted**                                                                                                                                                                                                                                                                                                                      | Already defined in 07                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| D54 | Fn annotation vs function body comparison missing (check green, runtime type error)         | **Declaration-driven checking mode.** Bound-time parameter types drive lambda header positionally, exit (tail expression and return) uniformly compared to annotation return type, registered type = annotation form; parser name merging, #295 append, `value_params` positional filling count precondition retired. Representation premise `Type::Fn.params: Vec<Param>` (name and type co-resident), landed in 6.8 of P6 | The comparison mechanism currently consists of three unconnected fragments (parser merge / registered annotation form / body check forks by syntax form), the invariant "declaration=implementation" is owned by no one — `f: () -> Int = () => "hello"` passes while `f: () -> Int = { "hello" }` errors (same semantics different syntax), check green, runtime E6007; this is a isomorphic onset of the proof_calls silent channel (§1.1) in the annotation subsystem. The comparison enablement batch is a behavior fix: new diagnostic, baseline update, C3 not applicable |

### Intermediate Representation (04)

| #   | Issue                                                               | **Decision**                                                                                                                                                                                     | Reason                                                                                                                                                                                                                                                                        |
| --- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D16 | `Instruction::Phi` static selection                                 | **Plan A: IR-only, `codegen` expands to Move sequence**                                                                                                                                          | Do not add new `PHI` opcode, avoid `.42` format expansion                                                                                                                                                                                                                     |
| D17 | `bytecode.rs:2312` `upvalue_count: 0`                               | **Fix; when adding new fields bump `.42` version number from 4 to 5** (format header already has `MAGIC` + `VERSION: u32` field, `codegen/bytecode.rs:14-16`, reading side validates by version) | This is a data loss defect not dead code; after version bump, old `.42` is rejected by the reading side per existing behavior (`.42` is a build product, cross-version compatibility is not a goal); already-assigned unused opcode values handled together with version bump |
| D18 | Whether u8/255 slot limit is sufficient after deletion              | **Must measure at end of batch d**; if not, expand to `u16`                                                                                                                                      | This is a verification item, not a pending item                                                                                                                                                                                                                               |
| D19 | Whether linear scan allocator should be prototyped first for timing | **No prototype, implement directly.** Timing measured in CI                                                                                                                                      | Timing does not change design decision, is procrastination                                                                                                                                                                                                                    |
| D20 | Span keying of `ReleasePlan` / `overload_resolutions`               | **Change to `PlanId`**, eliminate cross-layer span contract; attributed to P4                                                                                                                    | Span mismatch causes Drop silent loss, one of the most hidden defect types                                                                                                                                                                                                    |
| D21 | `method_def_ordinals` attributed to 03 or 04                        | **Attributed to 04** (P7 batch c)                                                                                                                                                                | It is IR construction-time state, not a type issue                                                                                                                                                                                                                            |
| D22 | Whether `synth.rs` boundary extends to the entire L3                | **Extend to the entire L3**; L4 will be included separately later                                                                                                                                | Boundary rules must be complete or invalid                                                                                                                                                                                                                                    |
| D23 | Whether `compile_pattern` / `eval_const_expr` is "not handled"      | **Include in batch d scope**, not exempt                                                                                                                                                         | Using "marked as not handled" to avoid work is patch thinking                                                                                                                                                                                                                 |

### Frontend (05)

| #   | Issue                                                                                   | **Decision**                                                                                                                                                                                                                                                                                                 | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D24 | Whether LALRPOP can fully reproduce existing diagnostics                                | **Must reproduce.** C5 not relaxed. Use explicit error productions to write the `synchronize()` synchronization point set and skip timing into the grammar                                                                                                                                                   | Not being able to do it is an implementation defect, **not a reason to relax the criteria**; report truthfully and re-evaluate (including "maintain Pratt"), not change the criteria                                                                                                                                                                                                                                                                                                 |
| D25 | Whether f-string span change in lexical phase is acceptable                             | **Not acceptable.** Span must record **absolute offset** in lexical phase, diff compare `span.file/line`                                                                                                                                                                                                     | Span change will affect diagnostic positioning, not acceptable                                                                                                                                                                                                                                                                                                                                                                                                                       |
| D26 | Whether associativity needs explicit declaration                                        | **Needed.** Explicitly declare left/right associativity in grammar, do not rely on `bp_right = bp_left + 1` convention                                                                                                                                                                                       | The current convention is exactly the source of magic numbers like `BP_RANGE` hardcoded to `(6,7)`                                                                                                                                                                                                                                                                                                                                                                                   |
| D27 | `is_old_function_syntax` (36 lines) disposition                                         | **Option 1: naturally delete with grammar migration.** `f(Int) -> Int = ...` cannot match any production                                                                                                                                                                                                     | Specifically probing removed syntax is an anti-pattern                                                                                                                                                                                                                                                                                                                                                                                                                               |
| D28 | Whether stage 0 deletes `pub use precedence::*;` in `pratt/mod.rs:11`                   | **Delete**                                                                                                                                                                                                                                                                                                   | It is the exposure surface of 96 lines of dead steps                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| D29 | 8-segment responsibility of `parse_assign_after_target`                                 | **All split, none retained.** `apply_semantic_side_effects` handed over to P6 but **not deleted**                                                                                                                                                                                                            | Splitting is responsibility re-division, not deletion of responsibility                                                                                                                                                                                                                                                                                                                                                                                                              |
| D30 | Stage 2-4 full corpus time consumption                                                  | **P2 baseline stage actually measured and recorded**                                                                                                                                                                                                                                                         | Is a verification item, not a pending item                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| D53 | What does the parameter position bare identifier (`Int` in `(Int, Int) -> Int`) read as | **Unnamed typed parameter.** Checker resolves by type namespace, reports E if type cannot be resolved; unnamed signature requires lambda header to carry its own parameter names (RFC-007 shorthand rules existing); RFC-010 form table and interface examples (`(Surface)`) reconciled. Landed in 8.9 of P8 | Parser currently reads as "infer parameter name" (`parse_fn_type_with_names` has no colon branch), annotation zero constraint, and violates RFC-007 §25 "both sides omitted will be rejected"; std interface layer (`ok: (T) -> Result(T, E)` etc.) and dozens of corpus evidence of bare form established reading is unnamed typed parameter, migration cost is zero; forcing `name: type` (veto) would migrate the entire std layer and prohibit the most intuitive way of writing |

### Cleanup and Criteria (06 / 07 / 08)

| #   | Issue                                                         | **Decision**                                                                                                                                                                                                                                                            | Reason                                                                                                       |
| --- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| D31 | Defects exposed by S1 reviving tests are fixed in which PR    | **Same PR**, test revival and defect fix together                                                                                                                                                                                                                       | Splitting will induce later generations to "fix next time"                                                   |
| D32 | `Switch` opcode (interpreter has implementation, no producer) | **Delete opcode and interpreter implementation.** Handled together with `.42` version bump (D17)                                                                                                                                                                        | An opcode without a producer is a dead path, whitelist only hides it                                         |
| D33 | `UnaryOp::Not` silently degrades to `I64_NEG`                 | **Fix.** `opcode()` must distinguish the `op` field, cannot ignore                                                                                                                                                                                                      | Silently changing semantics is one of the most dangerous types                                               |
| D34 | Encoding branch of `Instruction::TailCall`                    | **Delete** (no construction point in the entire repo)                                                                                                                                                                                                                   | Same reason as D32                                                                                           |
| D35 | cfg branches of wasm 27 files                                 | **Delete unreachable ones in playground scenario, keep reachable ones.** Attributed to P10                                                                                                                                                                              | wasm target already built (independent shim crate), branches are load-bearing, only delete unreachable parts |
| D36 | Whether `TRACKING.md` adds "Implementation Status" column     | **Add.** `check_tracking.py` adds generation logic                                                                                                                                                                                                                      | Current 26 accepted RFCs do not record implementation status at all, RFC-018 rotted this way                 |
| D37 | Dead reference in `docs/superpowers/` § (gitignored document) | **Delete comment**                                                                                                                                                                                                                                                      | Pointing to a gitignored planning document is a dead reference                                               |
| D38 | If `verify_loose` does not run green                          | **No exemption.** If it does not run green it means `ir_gen` has implicit "same slot multiple write" dependencies, **which are defects that must be fixed first**                                                                                                       | Exemption list is new technical debt, equals using gates to hide design problems                             |
| D39 | Snapshot baseline volume and compression                      | **Do not introduce git-lfs, accept volume**                                                                                                                                                                                                                             | Snapshots are regression criteria, readability over volume                                                   |
| D40 | Multi-file corpus layer                                       | **Unconditionally required** (2.3.2 of P2), not "skip and P4 on hold"                                                                                                                                                                                                   | Conditional clauses are backdoors for oneself                                                                |
| D41 | Whitelist of `test_release_plan_spans_consumed`               | **No whitelist, difference set must be empty** (this test is newly created in P2; the existing `WHITELIST = ["SWITCH"]` in repo belongs to opcode round-trip test, unrelated to this item). If not empty, ReleasePlan contract has a defect, fix the contract (see D20) | Whitelist legalizes bugs                                                                                     |
| D42 | `verify()` hooks to `cargo test` or CI                        | **CI**                                                                                                                                                                                                                                                                  | Full corpus verification time is not suitable for every local run                                            |
| D43 | Similarity algorithm for prohibition one A criterion          | **Jaccard ≥ 0.5**                                                                                                                                                                                                                                                       | Already set in 08 main text, delete open issue                                                               |
| D44 | Whether PR must declare responsibility attribution            | **Yes.** PR template adds required field                                                                                                                                                                                                                                | Responsibility determination cannot be machine-ized, can only be enforced by process                         |
| D45 | `// reason:` exemption reviewed by whom                       | **Merge into PR review checklist**, no dedicated person                                                                                                                                                                                                                 | With no team division, setting a dedicated person equals not setting                                         |

### Project and Process

| #   | Issue                                                             | **Decision**                                                                                                                                                                                                 | Reason                                                                                                                                                                                                                                                            |
| --- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D46 | `RFC-018` (accepted, 0 lines of code)                             | **Move back to `draft/`**                                                                                                                                                                                    | Per `rfc/index.md:131` definition "accepted = entering implementation phase", it has not                                                                                                                                                                          |
| D47 | Stage and Issue mapping                                           | **Each primary stage opens an Issue**, `check_tracking.py`'s `issues_impl` registers                                                                                                                         |                                                                                                                                                                                                                                                                   |
| D48 | Multi-file corpus layer location                                  | **`tests/yaoxiang-multifile/`** (new)                                                                                                                                                                        | Extending `multifile.rs` would mix semantic tests with contract corpora                                                                                                                                                                                           |
| D49 | Documentation site navigation                                     | **Add.** `config.js:255` "Tool Design" section add a new paragraph                                                                                                                                           | Without it, the nine companion documents are unreachable                                                                                                                                                                                                          |
| D50 | Stage parallelization (multi-file typecheck)                      | **Not this round.** As an independent issue after criteria stabilization                                                                                                                                     | Parallelization will mask order dependency defects, conflicts with this round's investigation goal                                                                                                                                                                |
| D51 | P3 bleeding stoppage channel                                      | **Allowed.** P3 prerequisites narrowed to 2.3.2 + 2.4.1 + 2.4.3; rest of P2 (IR validator / snapshot / single-file diff / performance baseline) can run in parallel with P3; P4 must wait for P2 to complete | The bleeding stoppage of correctness vulnerability (`Sorted(3)` silent pass) should not be blocked by snapshot infrastructure construction; P4 needs all three layers of criteria in place so not relaxed                                                         |
| D52 | `.42` three hardcoded discards (`bytecode.rs:2312`/`2315`/`2341`) | **All incorporated into P7 (7e), no "independent issue" left.** `2315` (exception table) and `2341` (global variables) were originally scheduled as independent issues, now incorporated                     | Exception table loss makes throw/try behave incorrectly in `.42` direct run — throw/try are language core semantics; this refactor does not leave core function unimplemented legacy. All three fixed in one `VERSION` bump (4→5), cheaper than two version bumps |

### Sole Remaining Data Item

| #   | Item                | Description                                                                                                                 |
| --- | ------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| —   | RFC's `issue` field | Filled: [#430](https://github.com/ChenXu233/YaoXiang/issues/430) (2026-10-05, tracking issue established on acceptance day) |

## Appendix: Glossary

| Term                                       | Definition                                                                                                                          |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **Stage**                                  | An exhaustible step in one compilation. The stage set must be an exhaustible enum, not runtime-registered                           |
| **Orchestration Layer (L1)**               | The layer that decides which stages run, how many times, and how failures propagate                                                 |
| **Frontend Layer (L2)**                    | Lexical and syntax, produces AST                                                                                                    |
| **Intermediate Representation Layer (L3)** | Type checking, static analysis, IR construction                                                                                     |
| **Execution Layer (L4)**                   | Bytecode generation, interpreter, runtime, std lib                                                                                  |
| **Obligation**                             | A contract item produced by some stage, must be consumed downstream or compilation fails                                            |
| **Obligations Ledger**                     | Container of all obligations, settled uniformly at the end of the stage table                                                       |
| **Orphan**                                 | A file never referenced by any `mod` declaration, therefore never participating in compilation                                      |
| **Shell**                                  | A file that has been declared, enters the compilation product, but only has documentation comments or unused `use`, zero assertions |
| **Equivalence Oracle**                     | Executable check proving behavior unchanged before and after refactor                                                               |
| **C1-C6**                                  | Refactor category grading, determines criterion strength. **No C5′ exists**                                                         |
| **Separation of Responsibilities**         | The only solution to scale problems: one module takes only one type of responsibility. **No line count / volume gate**              |
| **Decision Register**                      | The "Decision Register" section of this RFC, the final ruling on all open items                                                     |

## Appendix: Design Decision Log

| Decision                           | Determination                                                                                                                               | Date       | Recorder  |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------- |
| Layering granularity               | Four layers (L1-L4)                                                                                                                         | 2026-10-03 | ChenXu233 |
| Document structure                 | **1 RFC + 9 companion design documents** (`docs/src/dev/architecture/`, same level as `check/`, `formatter/`)                               | 2026-10-03 | ChenXu233 |
| Directory rename                   | **Do** (D1), no staging                                                                                                                     | 2026-10-03 | ChenXu233 |
| AST attribution                    | **Top-level `src/ast/`**, parallel to parser (industry majority; check-boundary exemption clause)                                           | 2026-10-04 | ChenXu233 |
| Opcode attribution                 | **`middle/bytecode/opcode.rs`**, merged with bytecode domain (vocabulary and format in same domain, dependency direction restored to L4→L3) | 2026-10-04 | ChenXu233 |
| Scale gate                         | **All canceled.** Scale solved by separation of responsibilities, adopt Go official position                                                | 2026-10-03 | ChenXu233 |
| Syntax paradigm                    | **Complete LALRPOP grammar-driven**; veto the "retain Pratt table-ization" intermediate state                                               | 2026-10-03 | ChenXu233 |
| Equivalence oracle                 | **C1-C6 six category grading, no relaxation** (no C5′ exists)                                                                               | 2026-10-03 | ChenXu233 |
| Deletion strategy                  | Allow complete deletion; orphan tests with real assertions **revive not delete**                                                            | 2026-10-03 | ChenXu233 |
| Verification method                | This set of documents does not execute cargo; based on static evidence + line numbers                                                       | 2026-10-03 | ChenXu233 |
| Stage order                        | P0 maintenance mechanism → P1 revive tests → P2 criteria → … → P10 cleanup                                                                  | 2026-10-03 | ChenXu233 |
| Open items                         | **50 items all locked down** (decision register D1-D50), no trade-offs left                                                                 | 2026-10-03 | ChenXu233 |
| P3 bleeding stoppage channel       | **Allowed** (D51), prerequisites narrowed to 2.3.2+2.4.1+2.4.3                                                                              | 2026-10-05 | ChenXu233 |
| `.42` three data loss points       | **All incorporated into P7 (7e), no independent issue** (D52)                                                                               | 2026-10-05 | ChenXu233 |
| Parameter position bare identifier | **Unnamed typed parameter** (D53), construction-time rejection falls on "type cannot be resolved"; with P8                                  | 2026-10-05 | ChenXu233 |
| Fn annotation comparison           | **Declaration-driven checking mode** (D54), representation premise `Type::Fn.params: Vec<Param>`; with P6                                   | 2026-10-05 | ChenXu233 |

## References

- [RFC-010 Unified Type Syntax](../accepted/010-unified-type-syntax.md) — Direct source of type
  representation convergence
- [RFC-011 Generic Type System](../accepted/011-generic-type-system.md)
- [RFC-011a Interface Implementation](../accepted/011a-interface-implementation.md)
- [RFC-013 Error Code Specification](../accepted/013-error-code-specification.md) — Example of
  generation-time gate
- [RFC-027 Compile-Time Evaluation and Types](../accepted/027-compile-time-evaluation-types.md)
- [RFC-029 Module Semantics](../accepted/029-module-semantics.md)
- [RFC-036 Test Framework](../accepted/036-test-framework.md)
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../draft/029a-module-cache-incremental.md)
- `src/frontend/core/typecheck/layers/README.md` — Current layer order declaration (opposite to
  actual, D7 requires fix)
- `build.rs:19-55` — Existing `panic!`-level gate example in this repo
- `docs/src/dev/design/check/` — Three documents describing an architecture that has never existed
  (see `06-cleanup-inventory.md` §C)
