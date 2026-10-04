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

This RFC proposes an architectural refactoring for the YaoXiang compiler (`src/` 160,002 lines / 509
`.rs` files): redraw boundaries around a **four-layer model** (orchestration / frontend /
intermediate representation / execution), establish an **exhaustive compile-stage contract** and an
**obligations ledger**, accompanied by a set of **equivalence oracles** and **rebound-prevention
gates**.

Detailed designs are placed in nine companion documents under
`docs/src/design/compiler-architecture/`. This RFC is responsible only for: the problem, root cause,
four-layer model, acceptance-oracle grading, and execution-stage sequencing. **Reviewing this RFC
alone is enough to decide whether to approve the project; the companion documents are the
construction blueprints after approval.**

## Motivation

### 1. Boundaries exist, but have no enforcement

The directory-level layering (`frontend` / `middle` / `backends`) is clear. But **the boundaries
exist only in people's self-awareness, with no mechanism to keep them from being eroded.** Three
direct consequences.

#### 1.1 Compile stages are hand-wired at five entry points, producing 11 behavioral inconsistencies

| Stage                      | Single-file `run` | Multi-file `run` | `check` | LSP (in-project) | LSP (single-file) |
| -------------------------- | ----------------- | ---------------- | ------- | ---------------- | ----------------- |
| lexing / parsing           | yes               | yes              | yes     | yes              | yes               |
| typecheck                  | yes               | yes              | yes     | yes              | yes               |
| **proof_execution**        | **yes**           | **no**           | **no**  | **no**           | **no**            |
| dead-code analysis         | yes               | no               | yes     | no               | no                |
| W1006 shadowing diagnostic | no                | no               | yes     | no               | no                |
| monomorphization           | yes               | no               | no      | no               | no                |
| IR generation / linking    | yes               | yes              | no      | no               | no                |

The five entry points: `src/frontend/pipeline.rs:141-227` (single-file, 5 stages at `149` / `161` /
`173` / `188` / `205`), `src/frontend/compiler.rs:106` (wrapper), four in
`src/frontend/module/orchestrator.rs` (`compile_project:99` / `check_project:273` /
`check_source_in_project:450` / `compile_embedded_module:1374`), and `src/lib.rs` `run_file:140` /
`run_project:154`.

The most severe cell causes a **correctness vulnerability**. Constraints on proof functions of the
form `x: Sorted(3)` **silently pass** under the multi-file, check, and LSP paths. Evidence chain
(all verifiable):

1. `src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** location in the entire
   repository that constructs a non-empty `proof_calls`; its semantics is "this constraint requires
   executing a proof function".
2. `src/frontend/core/typecheck/checker.rs` has three isomorphic branches handling `Unproven`:
   `5164` / `5306` / `5420` is `if calls.is_empty()` → push a hard error; `5179` / `5318` / `5448`
   is `ctx.proof_calls.extend(...)` → **produces no diagnostic at all**. The comment at `5165`
   literally says "Unproven → compile error, no degradation, no silent pass... the silent pass must
   not be resurrected."
3. `TypeCheckResult.proof_calls` (`types.rs:29`) is **read at exactly one location in the entire
   repository**: `src/frontend/pipeline.rs:187`.
4. None of the four entry points in `orchestrator.rs` read this field.

That is: invariants declared by the code itself are broken by the architecture rather than the
logic. The existence of `compile_embedded_module` (`orchestrator.rs:1374`) means **the refinement
obligations for the embedded std module itself also go through this discarding path**.

**No test can catch it**: the 27 tests in `tests/integration/multifile.rs` have zero hits for
`Sorted` / `proof` / `refin`; the unit tests in RFC-027 directly invoke `check_module` asserting
"`proof_calls` is non-empty", **stopping exactly before pipeline** — they verify "it was filled",
but the bug is "the consumer didn't read it".

#### 1.2 Module boundaries can be bypassed without trace

`src/frontend/core/typecheck/checker.rs:5618` is `include!("checker/semantic_tokens.rs");` — **the
only `include!` in the entire repository**. It splices 1547 lines of text into the `checker` module.
The file **isn't declared as any module** (`grep 'mod semantic_tokens'` has zero hits in the whole
repository), the first line is directly `impl TypeChecker {`, with no `use` header of its own, and
the `checker/` directory contains only this one file.

Consequences: no module identity, visibility isolation fails, rust-analyzer's go-to-definition and
symbol search fail on it, and the toolchain counts it as part of `checker.rs` during analysis. **The
true body of the `checker` module is 5618 + 1547 ≈ 7165 lines**, not the 5618 lines shown by
directory statistics.

#### 1.3 Test wiring relies on human memory; corruption happens silently

Running BFS reachability analysis of the module graph from the two crate roots `src/lib.rs` /
`src/main.rs`: **24 files / 2628 lines never participate in compilation**. Of these, 23 files / 1081
lines are real orphans.

| Orphan                                      | Size                 | Content                                                                              |
| ------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------ |
| Entire `frontend/core/lexer/tests/` subtree | 13 files / 686 lines | 7 empty shells totaling 19 lines + `mod.rs` 38 lines + **629 lines / 55 real tests** |
| `frontend/pipeline/tests/`                  | 3 files / 14 lines   | all placeholder documentation comments                                               |
| `package/template/tests/`                   | 3 files / 65 lines   | 7 `#[test]`                                                                          |
| `parser/pratt/tests/precedence_inline.rs`   | 95 lines / 6 tests   | hidden in a live directory                                                           |
| `typecheck/passes/tests/overload_inline.rs` | 170 lines / 7 tests  | same as above                                                                        |
| `util/diagnostic/emitter/tests/json.rs`     | 46 lines / 3 tests   | same as above                                                                        |

`cargo test` cannot discover them: tests that have never been run will never fail. CI
(`.github/workflows/ci.yml:140`) also has no test-count baseline or wiring check.

**Real tests that have never run: 1005 lines / 78 tests in total.** At the lexing layer, the only
test actually running is `fstring.rs` (connected via the `#[path]` attribute bypass).

### 2. Design capability is sufficient

It must be acknowledged at the same time: this project **already has** mature enforcement
mechanisms, but they haven't been generalized.

The error codes in `src/util/diagnostic/codes/` (137 E-codes + 8 W-codes = 145) are hard-gated at
build time by `build.rs:19-55` via `tools/code-tables` — it parses the registry, validates
uniqueness and segment positions, and compares item-by-item against the RFC-013 markdown code table;
**any inconsistency immediately triggers `panic!` and refuses compilation**. The RFC-013
documentation and code are therefore always consistent.

`src/package/` (76 files / 13,012 lines) is the most complete region: **tests are 6,227 lines,
47.9%**; 5 of the 6 `tests/` directories are correctly wired (the only exception is
`template/tests/`, see Section A). Note that this positive example holds at the **directory-level
wiring**, not at every file — of the 76 `.rs` files, only 7 contain inline `#[cfg(test)]` and 5
contain `mod tests;`. 5 RFCs match the code item-by-item; multiple "not implemented" entries have
been resolved into explicit errors rather than silent TODOs (`build/mod.rs:172-174` for cmake,
`error.rs:74-76` for `RegistryDeferred`). `.yxpkg` has five layers of safety protection, all
correct.

**In the same repository, with the same authors, `package/` does test-directory wiring right, and
`lexer/tests/` has never been run once. The difference is not in capability, but in whether someone
runs a check.**

### 3. Bogus designs

Three documents under `docs/src/design/check/` describe an architecture that **never existed**:
`CheckSession` (including the Rust code draft in `incremental-checking.md:31-41`),
`ModuleDependencyGraph`, `ModuleCache`, `HotReloader` — all have zero hits in the repository; the
`traits/` directory does not exist; the `check_single_module` function does not exist (it only lives
in test comments); even the `command.rs` busy-wait referenced in "known limitations" does not exist
(the file has neither `Instant` nor `recv_timeout`).

Cross-file analysis is actually implemented by `src/frontend/module/` (registry 439 + resolver 195 +
roles 388 + orchestrator 1528, about 2670 lines) — the capability is real, but the path is
completely different. **This document writes "designs that might exist in the future" as
"deficiencies in an already-implemented system".**

`src/frontend/pipeline/tests/compilation_cache.rs` and `incremental_scheduler.rs` (3 lines each) are
the cleanest physical evidence of "built the test skeleton to design, found no corresponding
implementation in the pipeline, then abandoned it".

In addition, `RFC-018` (LLVM AOT, 1037 lines, status: accepted) has 0 lines of implementation in the
code — all 6 AOT-related hits in `backends/mod.rs` are **comments**.

## Root cause diagnosis

Combining the three categories of symptoms, the root cause is singular:

> **This project treats "design" as a documentation convention, not as an executable constraint.**

| Layer           | Missing enforcement                                                | Existing but ungeneralized example                                               |
| --------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| Stage boundary  | A single decision point for "which stages ran in this compilation" | —                                                                                |
| Module boundary | Banning `include!`, restricting `pub(crate)` cross-layer leakage   | —                                                                                |
| Cross-reference | Detecting "a field is produced but consumed by no one"             | The error-code gate in `build.rs`                                                |
| Test wiring     | A `mod` declaration is required when a directory is declared       | `src/package/`'s directory-level wiring convention (5 of 6 `tests/` are correct) |

The position of this RFC is therefore: **turn the three classes of boundaries (stages, modules,
cross-references) from "documentation conventions" into "assertable facts at compile-time or
test-time"**, and **establish a unique stage decision point in the orchestration layer**.

## Proposal

### Four-layer model

```
┌─────────────────────────────────────────────────────────────┐
│ L1 Orchestration                                            │
│   Who decides which stages run, how many times,             │
│   and how failure propagates                                │
│   Current: pipeline.rs(740) + orchestrator.rs(1528)         │
│            + compiler.rs(267) = 5 entry points, 11 inconsistencies
│   Target:  single Driver + exhaustive Stage enum + Obligations ledger
│   See:     ../compiler-architecture/02-stage-contract.md      │
└───────────────────────────┬─────────────────────────────────┘
                            │ Compilation Unit (Unit)
┌───────────────────────────▼─────────────────────────────────┐
│ L2 Frontend                                                 │
│   Lexing, parsing, AST construction                         │
│   Current: lexer/(3065) + parser/(6341)                     │
│            hand-rolled lexer + hand-rolled Pratt;           │
│            adding an operator requires 6-7 changes          │
│            inside L2 + 17 production files downstream       │
│   Target:  lexing converges to a unique implementation     │
│            + LALRPOP grammar-driven syntax                  │
│   See:     ../compiler-architecture/05-frontend-paradigm.md   │
└───────────────────────────┬─────────────────────────────────┘
                            │ Module (AST)
┌───────────────────────────▼─────────────────────────────────┐
│ L3 Semantic & IR                                            │
│   Type checking, static analysis, IR construction           │
│   Current: typecheck/(29466 production) + ir.rs(905)       │
│            + ir_gen.rs(8448)                                │
│            3 parallel type representations                  │
│            + 3 parallel operator enums;                     │
│            ir_gen is a single 7952-line impl / 116 methods; │
│            no IR-level validator                            │
│   Target:  unified type representation;                     │
│            IR satisfies SSA construction discipline         │
│   See:     ../compiler-architecture/03-type-unification.md    │
│            ../compiler-architecture/04-ssa.md                 │
└───────────────────────────┬─────────────────────────────────┘
                            │ ModuleIR
┌───────────────────────────▼─────────────────────────────────┐
│ L4 Execution                                                │
│   Bytecode generation, interpreter, runtime, standard lib   │
│   Current: codegen/(3436) + bytecode.rs(2422)               │
│            + executor/(2000) + runtime/(1800) + std/(7543)  │
│   Target:  opcode is the single source of truth +           │
│            generation-time gate; L4 does not perceive L1-L3  │
│   See:     ../compiler-architecture/06-cleanup-inventory.md   │
└─────────────────────────────────────────────────────────────┘
```

**The four layers are strictly unidirectional**: L1 depends on the interfaces of L2/L3/L4; L2 does
not depend on L3; L3 does not depend on L2 (it only consumes AST data, does not call parser); L4
depends only on L3's product format.

The list of known reverse dependencies and parallel definitions is in routing table C of
`../compiler-architecture/01-routing.md`.

### Companion design documents

Nine documents, stored in `docs/src/design/compiler-architecture/`:

| Document                          | Layer            | Topic                                                                                                                                                                                                                                            |
| --------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `01-routing.md`                   | Cross-layer      | **Feature routing table**, dependency-direction specification, **target directory structure after construction**, future-extension guidance                                                                                                      |
| `02-stage-contract.md`            | L1               | Stage contract and obligations ledger: 11 inconsistencies, vulnerability evidence chain, `Stage` / `Obligations` / `Driver` / `ProgramKind` (including `WasmPlayground` on the wasm playground side)                                             |
| `03-type-unification.md`          | L2/L3 root cause | Type-representation unification: 26 `ast::Type` variants handled one-by-one, 13 forward zero-construction variants, type-table gate                                                                                                              |
| `04-ssa.md`                       | L3               | Intermediate-representation SSA-ization: four categories of defects, SSA form definition, register-allocator evaluation, 38-item change list                                                                                                     |
| `05-frontend-paradigm.md`         | L2               | Lexing converges to a unique implementation / syntax driven by LALRPOP grammar, change surface converges, dead-staircase handling, test reconstruction                                                                                           |
| `06-cleanup-inventory.md`         | Whole repo       | Dead-code and bogus-design cleanup: reachability method, per-item disposition, execution order                                                                                                                                                   |
| `07-equivalence-oracle.md`        | Cross-layer      | Equivalence oracle: three-layer oracle, C1-C6 grading, gate design                                                                                                                                                                               |
| **`08-maintenance-mechanism.md`** | **Cross-layer**  | **Repository maintenance mechanism**: three prohibitions (no fabrication / no endless filling / patching when refactoring is required), D0–D4 decision procedure, machine-checkable rules, code-review checklist, external-convention references |
| **`09-execution-wbs.md`**         | **Cross-volume** | **Multi-level construction WBS**: 11 first-level / 45 second-level / 122 third-level tasks, dependencies and parallel groupings, **8 conflict registrations**                                                                                    |

`01` and `09` are **long-term reference documents** — the former never becomes invalid due to any
refactor, and the latter is the construction checklist. The three prohibitions produced by `08`
constrain every action of P1–P10.

### Equivalence-oracle grading

**This is the safety net for all refactoring and must be established before any code change.**
Detailed design is in `07-equivalence-oracle.md`.

Core principle: **oracles are graded by refactor category, because different categories require
different strengths of equivalence.** Using a single oracle (usually "IR snapshot exact match") to
cover all refactors is wrong — some refactors **necessarily change the IR shape**.

| Category | Refactor content                                                               | Oracle type                                                         | Strength  |
| -------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------- | --------- |
| **C1**   | Pure relocation (splitting files, changing directories, extracting submodules) | IR canonicalized snapshot **zero-diff**                             | Strongest |
| **C2**   | Orchestration changes (staging, unifying Driver)                               | Each entry point has the same diagnostic set + same corpus behavior | Strong    |
| **C3**   | Type-representation convergence                                                | Diagnostic **codes** are identical (message wording will change)    | Strong    |
| **C4**   | IR-shape changes (SSA-ization)                                                 | **Behavioral equivalence** + IR structural invariants               | Medium    |
| **C5**   | Frontend paradigm change                                                       | AST snapshot + diagnostic + behavior, all three used                | Strong    |
| **C6**   | Pure deletion                                                                  | No equivalence needed, only confirmation of no references           | —         |

**The distinction between C1 and C4 is the key**: C1 requires IR to be byte-for-byte identical; C4
acknowledges that IR will change, and instead uses "the program's behavior is the same + IR
satisfies invariants". Forcing a snapshot on C4 will induce the team to relax the oracle.

Three layers of oracles:

1. **IR static validator** — dominant use-before-def, jump target existence, type consistency,
   inner-layer isolation. Far stronger than snapshots, and covers defects that snapshots cannot
   detect such as "IR is self-consistent but values are wrong". It must **first run green on the
   existing (non-SSA) IR**.
2. **Canonicalized IR snapshot** — strip `Span`, rename temporaries in occurrence order, relativize
   global slots, sort predecessors. Checked in, manually review diff. **Known limitation**:
   canonicalization will erase "which register was used for the N-th argument", so **it alone is
   insufficient to cover `arg_regs` semantic reshuffling**.
3. **End-to-end corpus diff** — diagnostic list, exit code, stdout/stderr item-by-item comparison.

> **⚠️ Structural limitation of corpus coverage**: there is **no `yaoxiang.toml`** under the
> `tests/` directory (verified: 0), so all **293** `.yx` corpora in `tests/yaoxiang/` go through the
> single-file path (`check_files_with_diagnostics`'s standalone branch → `check_single_file` →
> `Pipeline::run`).
>
> **Consequence**: the third-layer oracle **can only verify changes on the single-file path**. For
> the multi-file side (the four entry points of `orchestrator`) and the multi-file behavior of
> types/SSA, **there is currently no corpus coverage at all**. P4 "Unify Driver" is the largest risk
> point in the entire plan, and **the multi-file corpus layer must be established unconditionally in
> P2** (Resolution D40) — it is the only executable behavioral-oracle source for P4.

## Implementation strategy

### Stage sequence

```
P0  Repository maintenance mechanism     08        ← First establish rules: 3 prohibitions + D0-D4 decision procedure
P1  Revive orphan tests & fix defects   06 §S1   ← Cheapest, biggest payoff
P2  Build equivalence-oracle baseline   07 all
P3  Fix correctness vulnerability       02 §Fix
        (minimal solution)
P4  Stage contract & unified Driver     02
P5  checker file internal split         09 §P5 completion
P6  Type-representation unification    03
P7  Intermediate-representation SSA    04     } parallel
P8  Frontend paradigm change           05     }
P9  Rebound-prevention gates            01 + 08
        (unified script list)
P10 Remaining cleanup & status fixes   06 §S2-S6
```

**Core serial constraints (not interchangeable):**

| Constraint   | Reason                                                                                                                                                                                        |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1      | The three prohibitions constrain every action of P1 — 1.2.x is precisely adding `mod` declarations to `mod.rs`, which is exactly how the `precedence_inline.rs` life-and-death conflict arose |
| P1 → P2      | Reviving tests will change the test count and corpus baseline. Building the baseline first will invalidate it after P1                                                                        |
| P2 → P3      | The vulnerability oracle must be **written red first** to prove the fix is effective                                                                                                          |
| P3 → P4      | Fix the bug first, then refactor. Reversing the order will let the bug be solidified as "established behavior" by the stage table                                                             |
| P4 → P5      | P5 starts with `include!` → real `mod` (09 §P5 5.1); the subsequent checker split moves the same file and must be performed continuously after P4                                             |
| P5 → P6      | Consecutive changes to the same file must be separated, otherwise regression cannot be bisected                                                                                               |
| P6 → P7 / P8 | If type representation is not converged first, the new IR for SSA will grow into a third representation                                                                                       |
| P7/P8 → P9   | Initial baselines of each gate script (e.g., the `pub(crate)` leakage count) only take final values after P7/P8                                                                               |
| P7 ∥ P8      | Disjoint file sets, can run in parallel                                                                                                                                                       |

**Why P0 is a separate stage**: P9's CI scripts are **post-hoc checks** that can only block bad code
that has already been written; "patching when refactoring is required" is a **pre-judgment**, and
only the D0–D4 decision procedure can block it. The two cannot substitute for each other.

### Stage highlights

| Stage   | Scope of impact                                                                                                                                                                                                          | Acceptance                                                                                                                                                                                                                                 | Rollback point                                                                                         |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| **P0**  | `CONTRIBUTING.md` procedure section + 3 new gate scripts (`check-concepts` / `check-fanout` / `check-boundary`); **no compiler source changes, no line-count gates**                                                     | `check-concepts.py` **must report 3 parallel operator enums and 2 parallel type representations (including `ir::Type` aliases) on un-modified code**; deliberately exceeding limits or adding a 6th entry-point-style wiring must turn red | Pure addition; deleting the scripts is enough                                                          |
| **P1**  | Add `mod` declarations to 5 files; **revive 1005 lines / 78 tests**                                                                                                                                                      | Test count rises; **expected to expose real defects** (the `literals.rs` overflow path, the never-tested `\x` / `\u` illegal escapes)                                                                                                      | Per-file revert; **fixed defects should not be rolled back**                                           |
| **P2**  | New `verify.rs`, snapshot baseline, corpus-diff framework, **multi-file corpus layer**, performance baseline (criterion smoke benchmark), `scripts/ci/check-*`                                                           | `verify_loose` runs green; the vulnerability oracle is **written red**                                                                                                                                                                     | Oracle code can be removed wholesale                                                                   |
| **P3**  | `types.rs` (encapsulate `proof_calls` as private), three consumption points in `orchestrator.rs`                                                                                                                         | Vulnerability oracle **turns green**; **deliberately removing the fix must turn it red again**                                                                                                                                             | Pure behavior fix                                                                                      |
| **P4**  | New `src/driver/`; rewrite 5 entry points                                                                                                                                                                                | Same diagnostic set + same corpus behavior (C2)                                                                                                                                                                                            | **Largest risk point**; keep old entry points, an unwired Driver is enough to roll back                |
| **P5**  | `include!` → real `mod` (5.1); split `checker.rs` into `refinement` / `annotations`                                                                                                                                      | **IR snapshot zero-diff** (C1); the `pub(crate)` path at `statements.rs:16` is unchanged                                                                                                                                                   | Per-file revert                                                                                        |
| **P6**  | `ast.rs`, `types/mono.rs`, `solver.rs`, `ir.rs:3`, `bytecode.rs:2353-2390`; end-of-stage execution of directory renames (D1: `typecheck/`→`sema/`, `middle/core/`→`middle/ir/`); same for `ir_gen.rs` consumers          | Same diagnostic **codes** (C3); the rename batch is pure relocation (C1 zero-diff)                                                                                                                                                         | Phased commits, variant handling separated from synonym-table deletion; rename occupies its own commit |
| **P7**  | `ir.rs`, `ir_gen.rs` (8448 lines), `bytecode.rs`, `translator.rs`                                                                                                                                                        | **Behavioral equivalence + `verify_ssa` green** (C4). **Not IR snapshot equality**                                                                                                                                                         | Per-batch revert                                                                                       |
| **P8**  | `lexer/*`, `parser/*`                                                                                                                                                                                                    | AST snapshot + diagnostic + behavior (C5)                                                                                                                                                                                                  | Per-file revert                                                                                        |
| **P9**  | 10 new `scripts/ci/check-*.py` (the unique list is in [09](../../compiler-architecture/09-execution-wbs.md) §P9: 3 introduced with P0, 1 with P1, 3 with P2, 2 with P4, 1 with P7, P9 consolidates and switches to hard) | **Deliberately leaving an obligation with no consumer must turn red**; **deliberately creating an orphan test directory must turn red**; **deliberately adding `include!` must turn red**                                                  | Delete scripts                                                                                         |
| **P10** | Documentation disposition, RFC status fixes, reducing `pub` item visibility, opcode decisions                                                                                                                            | `check_tracking.py` passes; no 404 in the doc site                                                                                                                                                                                         | Independent PR                                                                                         |

### Definition of Done (DoD) per stage

A stage is considered complete when all of the following are met simultaneously:

1. All corresponding "implementation strategy" steps in the owning document are committed
2. The oracle for that category (C1-C6) in `07-equivalence-oracle.md` **is green in CI**
3. `cargo test` is fully green, and **the test count is not less than at the start of the stage**
   (preventing "delete tests for a green light")
4. `cargo clippy --all --all-features -- -D warnings` is clean
5. `python scripts/rfc/check_tracking.py` exits with code 0
6. The stage has an independent revert unit — **if a stage cannot be reverted on its own, it is too
   large and must be split further**
7. For stages that touch the compilation pipeline or execution path (P4 / P6 / P7 / P8), the 07
   performance baseline comparison has no unexplained >10% regression

### Global acceptance gates

After all stages are complete, the following must be true:

| #   | Condition                                           | Verification                                                                           |
| --- | --------------------------------------------------- | -------------------------------------------------------------------------------------- |
| G1  | The decision point for compile stages is unique     | Answering "which stages ran, which didn't, and why" requires looking at only one place |
| G2  | No "produced but unconsumed" fields                 | `check-obligations.py` green                                                           |
| G3  | `include!` count is 0                               | `grep -rn 'include!' src/`                                                             |
| G4  | No orphan tests                                     | `check-test-wiring.py` green                                                           |
| G5  | No reverse dependencies                             | `check-module-boundary.py` green                                                       |
| G6  | Type representation is unique                       | Parallel representations converged; hand-written synonym tables gone                   |
| G7  | parser contains no type/predicate hardcoding        | String literals like `"Terminates"` replaced by data flow                              |
| G8  | Adding a binary operator changes ≤ 3 places         | Manual review + routing-table update                                                   |
| G9  | `layers/README.md` describes the actual layer order | Document consistent with `check_module_impl`                                           |
| G10 | `TRACKING.md` has an "implementation status" column | Implementation status of 52 RFCs is queryable                                          |

## Trade-offs

### Advantages

- **Gives all subsequent changes a landing point**. "Adding an operator requires changes in a dozen
  places, and nobody knows if one is missed" cannot be improved by memory, only by routing tables +
  gates.
- **Single root cause**. The three categories of symptoms (stage inconsistency, `include!`, test
  wiring breakage) all stem from "boundaries are conventions not constraints", and can be solved by
  the same kind of mechanism in one go.
- **Non-blocking**. This RFC makes zero code changes. The four-layer model is descriptive; after
  adoption, only part of it can be executed.
- **Oracle first**. P2 is established before any code changes, and the graded design avoids the
  common failure of "snapshot too strict → induce relaxation".

### Disadvantages and risks

- **The serial chain is long**. P1 → P2 → P3 → P4 → P5 → P6 → P7/P8 is nearly fully serial; later
  stages have to wait for earlier ones to complete.
- **P1 will introduce new bug workload**, and this is the step most easily skipped with the
  rationale "we need to fix bugs so let's not do this first" — but skipping it means all
  subsequent-stage acceptance oracles rest on a false coverage foundation.
- **P4 is the single largest risk point**: one change touches 5 entry points.
- **P6 has the broadest impact surface**: touching the common dependencies of parser, formatter,
  spawn, and orchestrator.
- **P7 will have a net increase of 900-1600 lines, not a net decrease**. Splitting introduces
  boilerplate; `verify` is 300-600 lines, register allocation is 600-1000 lines. **If the project is
  judged by "code gets shorter" at approval, this stage will be judged as a failure** — this point
  must be aligned at approval.

## Alternatives

### A. Only split files, do not change paradigms

Split 8448 lines into 10 files of 800 lines each.

**Not adopted.** Splitting doesn't eliminate any root cause: the 6 hand-rolled save/restore sites
are still scattered across 10 files, the `arg_regs` semantic reshuffling in `generate_call_expr_ir`
still requires manual reasoning, and "adding an operator changes a dozen places" still changes a
dozen places. Splitting improves navigation, not correctness.

### B. Only add gates, do not change structure (including line-count ratchet)

**Not adopted as the sole approach, but it is part of this RFC.** Gates can prevent regression, but
cannot fix the current state — `ir_gen.rs` is already 8448 lines, and a gate can only freeze it at
that number.

### C. Start over, write a new compiler skeleton

**Not adopted.** `src/package/` proves that this team can do complex systems right. The problem
isn't capability. Also, the existing 293 `.yx` corpora provide a ready-made equivalence-oracle
source; rewriting from scratch would waste this asset.

### D. Use an existing compiler framework (e.g., LLVM as the only backend)

**Not adopted.** The `Executor` trait (`backends/mod.rs:356`) already reserves an abstraction point
but has only one implementer. Introducing LLVM is in the scope of `RFC-018` (1037 lines of design, 0
lines of code) and should be **a separate decision** — it changes "what to compile to", not "how to
organize the code".

## Resolution registry: all open items are settled

> **This section is the sole authority.** The original `- [ ]` open items and "conflict
> registrations" in each companion document **all converge to this table** and are no longer kept
> individually. **There is no "pending", no "deferred", no "optional".** Each entry gives the
> decision and reason. If during implementation a decision is found to have a technical premise that
> no longer holds, **the correct action is to come back to this table to change the decision and
> explain why**, not to bypass it.

### Directories and naming

| #   | Topic                                                                                                                                                                         | **Decision**                                                                                                                                             | Reason                                                                                                                                                                                                                                                                                                                                                                                  |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Whether to rename directories (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`, move `parser/ast.rs` to top-level `src/ast/`, move opcode vocabulary to `middle/bytecode/`) | **Do it.** No phasing, no "optional"; complete with P5/P6 (top-level AST domain and bytecode domain merge in the same batch, all are C1 pure relocation) | Boundaries are guaranteed by CI, but **directory names are the first-glance signal of responsibility**. `middle/core/` holds `ir.rs` + `ir_gen.rs` + `bytecode.rs` (three things); the name is already invalid. AST is consumed by all layers; placing it under `frontend/` misleads downstream paths (6/7 of industry peers keep it as a top-level domain at the same level as parser) |
| D2  | Naming convention                                                                                                                                                             | Keep `type_.rs` / `fn_.rs` (avoid Rust 2024 reserved words); prohibit `xxx_v2` / `xxx_new` directories                                                   | Parallel copies are a common source of dead code (the `Precedence` enum in `pratt/precedence.rs` runs parallel to the actually-effective BP constants, with zero production references; `Expr::FnDef` is a parallel construction path that is never executed)                                                                                                                           |

### Stage contract (02)

| #   | Topic                                                                                                                                                                                                                                             | **Decision**                                                                                                                                      | Reason                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D3  | Whether to merge `check_module` (`typecheck/mod.rs:81`, fail-fast; early-exit implementation at `inference/statements.rs:2797-2799`) vs `check_module_collect_all` (`typecheck/mod.rs:93`, collect-all; called at `orchestrator.rs:124` / `:493`) | **Do not merge.** Keep both APIs, driven by the `Program` aggregate mode field (`Aggregation: FailFast \| CollectAll`, see 02)                    | The two have genuinely different semantics (`CollectAll` serves LSP). Forcing a merge will introduce `Option` noise                                                                                 |
| D4  | Which of the 16 fields in `Obligations` are "must be consumed"                                                                                                                                                                                    | **All 16 go into the ledger.** Fields with no consumer (e.g., `module_namespaces`) are removed together in P4's S4, **no orphan fields left**     | The point of a ledger is no exceptions                                                                                                                                                              |
| D5  | Diagnostic level of `assert_drained()`                                                                                                                                                                                                            | **Final E-level** (error); within P4 first observe with W-level, then upgrade to E at the end of the same stage (the two-step approach of 02 §S4) | Staying at W long-term means no gate — W doesn't change the exit code; but jumping straight to E makes the C2 oracle unusable, so E is the final state and W is only a transitional state within P4 |
| D6  | `ownership.rs:627` creates a new Z3Backend every time                                                                                                                                                                                             | **Change to singleton**, unified with the global `LazyLock` at `predicate.rs:34-36` as `SolverProvider`                                           | Three acquisition strategies, two failure philosophies (one panics, one is silent) must converge                                                                                                    |
| D7  | `layers/README.md` layer order is opposite to reality                                                                                                                                                                                             | **Fix**, attributed to P4's 4.4.1                                                                                                                 | The README describes an intent architecture that hasn't been built; the fix will expose the previously-silent `Unproven`, **which should be exposed**                                               |

### Type representation (03)

| #   | Topic                                                                                           | **Decision**                                                                                                                                                                                                      | Reason                                                                                                                                       |
| --- | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| D8  | Whether to delete `AssocType` (`ast.rs:463-471`)                                                | **Delete.** Zero production constructions (the only construction in the entire repository is in the test `types/tests/mono.rs:178`, which doesn't change the conclusion), reviewed independently with the T1 gate | Same handling as the other 11 zero-construction variants; if associated-type syntax is enabled in the future, redefine it via a new proposal |
| D9  | T1 gate uses `syn` or Python                                                                    | **Use `syn`.** `tools/code-tables` is already a Rust crate; reuse it                                                                                                                                              | Building another Python parser is a re-implementation and violates prohibition one                                                           |
| D10 | What to change `Type::Void` fallback (`parser/statements/types.rs:836`) to                      | **Change to `Err(Diagnostic)` return**                                                                                                                                                                            | A silent fallback is a kind of "fabrication" — using a fake value to mask absence                                                            |
| D11 | `NameKind` determination + LSP path probe                                                       | **LSP reuses `TypeEnvProbe`**, no separate probe channel                                                                                                                                                          | `lsp/world.rs` already holds `SemanticDB`, low cost to reuse                                                                                 |
| D12 | Who generates `const_data::BinOp` / `ast::BinOp`                                                | **Take `ast::BinOp` as canonical**, `const_data` side aligns by semantics (don't force renaming `Neq`/`Ne`)                                                                                                       | AST is the primary representation; renaming would ripple to 12 references without benefit                                                    |
| D13 | `classify_generic_params` uses `signature_params` or `Lambda.params`; same for `ir_gen.rs:1380` | **Not an open question, this is a mandatory verification item of P6 stage 6.1.3**, conclusion is written into the PR                                                                                              | This is an execution step, not a pending strategy                                                                                            |
| D14 | Non-canonical type-name statistics in 293 corpora                                               | **Same as above, this is a task** (6.1.1), not an open question                                                                                                                                                   | Same as above                                                                                                                                |
| D15 | Classification of "no behavior change" vs "behavior change" in stage 2                          | **Classify by C3 oracle: design changes go in independent commits, wording changes do not count**                                                                                                                 | Already defined in 07                                                                                                                        |

### Intermediate representation (04)

| #   | Topic                                                           | **Decision**                                                                                                                                                                                                | Reason                                                                                                                                                                                                                                                                                         |
| --- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D16 | Static selection of `Instruction::Phi`                          | **Plan A: IR-only, `codegen` expands to a Move sequence**                                                                                                                                                   | Do not add a `PHI` opcode, avoid further expansion of the `.42` format                                                                                                                                                                                                                         |
| D17 | `bytecode.rs:2312` `upvalue_count: 0`                           | **Fix; bump the `.42` version number from 4 to 5 when adding the field** (the format header already has `MAGIC` + `VERSION: u32` fields, `codegen/bytecode.rs:14-16`; the reader side validates by version) | This is a data-loss defect, not dead code; after the version bump, the old `.42` is rejected by the existing reader-side behavior (`.42` is a build artifact, cross-version compatibility is not a goal); already-assigned but unused opcode values are handled together with the version bump |
| D18 | After deletion, is the u8/255 slot upper limit sufficient       | **Must be measured at the end of batch d**; if not, expand to `u16`                                                                                                                                         | This is a verification item, not a pending item                                                                                                                                                                                                                                                |
| D19 | Does the linear-scan allocator require a prototype for timing   | **Do not prototype, implement directly.** Time is measured in CI                                                                                                                                            | Timing doesn't change the design decision; it's procrastination                                                                                                                                                                                                                                |
| D20 | Span keying for `ReleasePlan` / `overload_resolutions`          | **Change to `PlanId`**, eliminate the cross-layer span contract; attributed to P4                                                                                                                           | Span mismatch causes Drop to be silently lost, which is the most insidious type of defect                                                                                                                                                                                                      |
| D21 | Whether `method_def_ordinals` belongs in 03 or 04               | **04** (P7 batch c)                                                                                                                                                                                         | It's IR-construction-time state, not a type issue                                                                                                                                                                                                                                              |
| D22 | Whether the `synth.rs` boundary is generalized to the entire L3 | **Generalize to the entire L3**; L4 investigated separately and folded in later                                                                                                                             | Boundary rules are either complete or invalid                                                                                                                                                                                                                                                  |
| D23 | Whether `compile_pattern` / `eval_const_expr` are "not handled" | **Included in batch d scope**, not exempt                                                                                                                                                                   | Using "marked as not handled" to evade work is patch-think                                                                                                                                                                                                                                     |

### Frontend (05)

| #   | Topic                                                                         | **Decision**                                                                                                                                            | Reason                                                                                                                                                                  |
| --- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D24 | Whether LALRPOP can fully reproduce existing diagnostics                      | **Must reproduce.** C5 is not relaxed. Use explicit error productions to write the `synchronize()` synchronization set and skip timing into the grammar | Inability to do so is an implementation defect, **not a reason to relax the oracle**; report truthfully and re-evaluate (including "keep Pratt"), not change the oracle |
| D25 | Whether the f-string span change when determined at lexing time is acceptable | **Not acceptable.** Span must record the **absolute offset** at lexing time; diff compares `span.file/line`                                             | Span changes affect diagnostic location and are unacceptable                                                                                                            |
| D26 | Whether associativity needs to be declared explicitly                         | **Yes.** Left/right associativity is declared explicitly in the grammar, not relying on the `bp_right = bp_left + 1` convention                         | The current convention is exactly the source of magic numbers like the hardcoded `(6,7)` in `BP_RANGE`                                                                  |
| D27 | Disposition of `is_old_function_syntax` (36 lines)                            | **Option 1: delete naturally with the grammar migration.** `f(Int) -> Int = ...` cannot match any production                                            | Specifically probing for already-removed syntax is an anti-pattern                                                                                                      |
| D28 | Whether to delete `pub use precedence::*;` at `pratt/mod.rs:11` in stage 0    | **Delete**                                                                                                                                              | It's the exposure surface of the 96-line dead staircase                                                                                                                 |
| D29 | The 8-segment responsibilities of `parse_assign_after_target`                 | **All split, none kept.** `apply_semantic_side_effects` is handed off to P6 but **not deleted**                                                         | Splitting is responsibility re-division, not deletion of responsibility                                                                                                 |
| D30 | Full-corpus time consumption for stages 2-4                                   | **Measured and recorded in the P2 baseline stage**                                                                                                      | This is a verification item, not a pending item                                                                                                                         |

### Cleanup and oracle (06 / 07 / 08)

| #   | Topic                                                            | **Decision**                                                                                                                                                                                                                                                                                 | Reason                                                                                                                   |
| --- | ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| D31 | In which PR to fix the defects exposed by reviving S1 tests      | **The same PR**, test revival and defect fix together                                                                                                                                                                                                                                        | Splitting them will tempt future contributors to "fix it next time"                                                      |
| D32 | `Switch` opcode (interpreter has an implementation, no producer) | **Delete the opcode and the interpreter implementation.** Handled together with the `.42` version bump (D17)                                                                                                                                                                                 | An opcode with no producer is a dead path; a whitelist just conceals it                                                  |
| D33 | `UnaryOp::Not` silently degrades to `I64_NEG`                    | **Fix.** `opcode()` must distinguish the `op` field and cannot ignore it                                                                                                                                                                                                                     | Silently changing semantics is the most dangerous type                                                                   |
| D34 | Encoding branch of `Instruction::TailCall`                       | **Delete** (no construction site in the entire repository)                                                                                                                                                                                                                                   | Same reasoning as D32                                                                                                    |
| D35 | 27-file cfg branches in wasm                                     | **Delete unreachable branches in the playground scenario, keep reachable ones.** Attributed to P10                                                                                                                                                                                           | The wasm target is built (independent shim crate); the branches are load-bearing, only the unreachable parts are deleted |
| D36 | Whether `TRACKING.md` adds an "implementation status" column     | **Add.** `check_tracking.py` adds generation logic                                                                                                                                                                                                                                           | The current 26 accepted RFCs completely don't record implementation status; that's how RFC-018 rotted                    |
| D37 | §-dead references in `docs/superpowers/` (gitignored documents)  | **Delete the comments**                                                                                                                                                                                                                                                                      | Pointing to a gitignored planning document is a dead reference                                                           |
| D38 | If `verify_loose` cannot run green                               | **No exemptions.** If it cannot run green, it means `ir_gen` has an implicit "same-slot multiple-write" dependency, **which is a defect that must be fixed first**                                                                                                                           | The exemption list is new technical debt — it's using a gate to cover up a design problem                                |
| D39 | Snapshot baseline size and compression                           | **Do not introduce git-lfs, accept the size**                                                                                                                                                                                                                                                | Snapshots are regression oracles; readability takes precedence over size                                                 |
| D40 | Multi-file corpus layer                                          | **Unconditionally required** (P2's 2.3.2), not "P4 deferred if not done"                                                                                                                                                                                                                     | Conditional clauses leave a back door open for oneself                                                                   |
| D41 | Whitelist for `test_release_plan_spans_consumed`                 | **No whitelist, the difference set must be empty** (this test is newly created in P2; the existing `WHITELIST = ["SWITCH"]` in the repository belongs to opcode round-trip tests and is unrelated). If it cannot be empty, the ReleasePlan contract is defective; fix the contract (see D20) | A whitelist is legalizing a bug                                                                                          |
| D42 | Whether `verify()` hooks into `cargo test` or CI                 | **CI**                                                                                                                                                                                                                                                                                       | Full-corpus validation time is not suitable for every local run                                                          |
| D43 | Similarity algorithm for prohibition one criterion A             | **Jaccard ≥ 0.5**                                                                                                                                                                                                                                                                            | Already fixed in `08` body, delete the open question                                                                     |
| D44 | Whether the PR must declare responsibility attribution           | **Yes.** Add a required field to the PR template                                                                                                                                                                                                                                             | Responsibility determination is not machine-checkable and can only be enforced by process                                |
| D45 | Who reviews `// reason:` exemptions                              | **Folded into the PR review checklist**, no dedicated person                                                                                                                                                                                                                                 | With no team division, designating a dedicated person is equivalent to designating no one                                |

### Project and process

| #   | Topic                                    | **Decision**                                                                                    | Reason                                                                                              |
| --- | ---------------------------------------- | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| D46 | `RFC-018` (accepted, 0 lines of code)    | **Move back to `draft/`**                                                                       | Per the definition at `rfc/index.md:131`, "accepted = entering the implementation phase"; it is not |
| D47 | Stage-to-Issue mapping                   | **Open one Issue per first-level stage**; the `issues_impl` of `check_tracking.py` registers it |                                                                                                     |
| D48 | Multi-file corpus layer location         | **`tests/yaoxiang-multifile/`** (newly created)                                                 | Extending `multifile.rs` will mix semantic tests with contract corpora                              |
| D49 | Documentation site navigation            | **Add.** Add a new section beside "Tool Design" at `config.js:255`                              | Without it, the nine companion documents are unreachable                                            |
| D50 | Stage parallelism (multi-file typecheck) | **Not done in this round.** Stable oracle is a prerequisite, and will be a separate topic       | Parallelism will mask order-dependence defects, conflicting with this round's investigation goals   |

### Sole remaining data-fill item

| #   | Item                | Description                                                                                                                                                                                        |
| --- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| —   | RFC's `issue` field | Currently `TBD`. **A real Issue number must be filled in before submitting the PR**, otherwise `check_tracking.py` can pass, but `issue_rfc_link` in `scripts/rfc/ai_agent.py` cannot associate it |

## Appendix: Glossary

| Term                                       | Definition                                                                                                                                   |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage**                                  | An exhaustively enumerable step in one compilation. The set of stages must be an exhaustive enumeration; runtime registration is not allowed |
| **Orchestration layer (L1)**               | The layer that decides which stages run, how many times, and how failure propagates                                                          |
| **Frontend layer (L2)**                    | Lexing and syntax, producing AST                                                                                                             |
| **Intermediate-representation layer (L3)** | Type checking, static analysis, IR construction                                                                                              |
| **Execution layer (L4)**                   | Bytecode generation, interpreter, runtime, standard library                                                                                  |
| **Obligation**                             | A contract item produced by some stage that must be consumed downstream or compilation fails                                                 |
| **Obligations ledger**                     | The container of all obligations, settled uniformly at the tail of the stage table                                                           |
| **Orphan**                                 | A file never referenced by any `mod` declaration, therefore never participating in compilation                                               |
| **Shell**                                  | A file that is declared, enters the compilation product, but has only documentation comments or unused `use`, with zero assertions           |
| **Equivalence oracle**                     | An executable check that proves behavior is unchanged before and after a refactor                                                            |
| **C1-C6**                                  | Refactor category grading, determining oracle strength. **There is no C5′**                                                                  |
| **Responsibility separation**              | The only solution to scale problems: a module takes on only one type of responsibility. **No line-count / volume gates**                     |
| **Resolution registry**                    | The "Resolution Registry" section of this RFC; the final ruling on all open items                                                            |

## Appendix: Design-decision log

| Decision            | Decision                                                                                                                                            | Date       | Recorder  |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------- |
| Layer granularity   | Four layers (L1-L4)                                                                                                                                 | 2026-10-03 | ChenXu233 |
| Document structure  | **1 RFC + 9 companion design documents** (`docs/src/design/compiler-architecture/`, at the same level as `check/`, `formatter/`)                    | 2026-10-03 | ChenXu233 |
| Directory renaming  | **Do it** (D1), no phasing                                                                                                                          | 2026-10-03 | ChenXu233 |
| AST ownership       | **Top-level `src/ast/`**, at the same level as parser (industry majority; check-boundary free-exemption clause)                                     | 2026-10-04 | ChenXu233 |
| Opcode ownership    | **`middle/bytecode/opcode.rs`**, merged with the bytecode domain (vocabulary and format in the same domain, dependency direction restored to L4→L3) | 2026-10-04 | ChenXu233 |
| Scale gates         | **All cancelled.** Scale is solved by responsibility separation, adopting Go's official stance                                                      | 2026-10-03 | ChenXu233 |
| Syntax paradigm     | **Full LALRPOP grammar-driven**; reject the "keep Pratt with table lookup" middle ground                                                            | 2026-10-03 | ChenXu233 |
| Equivalence oracle  | **Six C1-C6 graded categories, no relaxation** (there is no C5′)                                                                                    | 2026-10-03 | ChenXu233 |
| Deletion strategy   | Allowed to delete thoroughly; orphan tests with real assertions are **revived rather than deleted**                                                 | 2026-10-03 | ChenXu233 |
| Verification method | This set of documents does not run cargo; uses static evidence + line numbers                                                                       | 2026-10-03 | ChenXu233 |
| Stage order         | P0 maintenance mechanism → P1 revive tests → P2 oracle → … → P10 cleanup                                                                            | 2026-10-03 | ChenXu233 |
| Open items          | **All 50 items locked in** (Resolution Registry D1-D50), no trade-offs left                                                                         | 2026-10-03 | ChenXu233 |

## References

- [RFC-010 Unified Type Syntax](../accepted/010-unified-type-syntax.md) — The direct source of
  type-representation convergence
- [RFC-011 Generic Type System](../accepted/011-generic-type-system.md)
- [RFC-011a Interface Implementation](../accepted/011a-interface-implementation.md)
- [RFC-013 Error Code Specification](../accepted/013-error-code-specification.md) — An example of
  generation-time gates
- [RFC-027 Compile-Time Evaluation and Types](../accepted/027-compile-time-evaluation-types.md)
- [RFC-029 Module Semantics](../accepted/029-module-semantics.md)
- [RFC-036 Test Framework](../accepted/036-test-framework.md)
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](029a-module-cache-incremental.md)
- `src/frontend/core/typecheck/layers/README.md` — Current declared layer order (opposite of
  reality; D7 requires correction)
- `build.rs:19-55` — The existing `panic!`-level gate example in this repository
- `docs/src/design/check/` — The three documents describing an architecture that never existed (see
  `06-cleanup-inventory.md` §C)
