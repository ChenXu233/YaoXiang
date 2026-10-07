---
title: 'RFC-039: Compiler Architecture Refactoring (Master Plan)'
status: 'accepted'
author: 'ChenXu233'
created: '2026-10-03'
updated: '2026-10-05'
accepted: '2026-10-05'
issue: '#430'
---

# RFC-039: Compiler Architecture Refactoring

## Summary

This RFC proposes an architectural refactoring for the YaoXiang compiler (`src/` 160,002 lines / 509
`.rs` files): redraw boundaries with a **four-layer model** (Orchestration / Frontend / Intermediate
Representation / Execution), establish an **exhaustively-enumerable compilation-stage contract** and
an **obligations ledger**, and pair them with a set of **equivalence oracles** and
**rebound-prevention gates**.

The detailed design lives in nine companion documents under `docs/src/dev/architecture/`. This RFC
is responsible only for: the problem, the root cause, the four-layer model, the graded acceptance
criteria, and the execution-stage order. **Reviewing this RFC alone is sufficient to decide whether
to launch the project; the companion documents are the construction blueprint after launch.**

## Motivation

### I. Boundaries exist, but have no enforcement

The directory-level layering (`frontend` / `middle` / `backends`) is clear. But **the boundaries
only exist in human self-discipline, with no mechanism to prevent them from being eroded.** Three
direct consequences.

#### 1.1 Compilation stages are hand-wired across five entry points, producing 11 behavioral inconsistencies

| Stage                      | Single-file `run` | Multi-file `run` | `check` | LSP (project) | LSP (single-file) |
| -------------------------- | ----------------- | ---------------- | ------- | ------------- | ----------------- |
| lexing / parsing           | yes               | yes              | yes     | yes           | yes               |
| typecheck                  | yes               | yes              | yes     | yes           | yes               |
| **proof_execution**        | **yes**           | **no**           | **no**  | **no**        | **no**            |
| dead-code analysis         | yes               | no               | yes     | no            | no                |
| W1006 shadowing diagnostic | no                | no               | yes     | no            | no                |
| monomorphization           | yes               | no               | no      | no            | no                |
| IR generation / linking    | yes               | yes              | no      | no            | no                |

The five entry points: `src/frontend/pipeline.rs:141-227` (single-file, 5 stages at `149` / `161` /
`173` / `188` / `205`), `src/frontend/compiler.rs:106` (wrapper), the four in
`src/frontend/module/orchestrator.rs` (`compile_project:99` / `check_project:273` /
`check_source_in_project:450` / `compile_embedded_module:1374`), and `src/lib.rs`'s `run_file:140` /
`run_project:154`.

The worst cell causes a **correctness vulnerability**. Constraints on proof functions of the form
`x: Sorted(3)` silently pass on the multi-file, check, and LSP paths. Evidence chain (all
reproducible):

1. `src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the entire
   repository that constructs a non-empty `proof_calls`, with the semantics "this constraint
   requires executing the proof function".
2. `src/frontend/core/typecheck/checker.rs` has three isomorphic branches handling `Unproven`:
   `5164` / `5306` / `5420` are `if calls.is_empty()` → push hard error; `5179` / `5318` / `5448`
   are `ctx.proof_calls.extend(...)` → **produces no diagnostic**. The comment at `5165` literally
   says: "Unproven → compile error, no degradation, no silent pass … silent pass must not be
   resurrected."
3. `TypeCheckResult.proof_calls` (`types.rs:29`) has **only one read site in the entire repository:
   `src/frontend/pipeline.rs:187`**.
4. The four entry points in `orchestrator.rs` **all ignore this field**.

That is: an invariant declared by the code itself is broken by architecture, not by logic. The
existence of `compile_embedded_module` (`orchestrator.rs:1374`) means that **the refinement
obligations of the embedded std module itself also go through this discarding path**.

**No test can catch it:** the 27 tests in `tests/integration/multifile.rs` have zero hits for
`Sorted` / `proof` / `refin`; the unit tests in RFC-027 directly call `check_module` and assert that
"`proof_calls` is non-empty", which **stops exactly before the pipeline** — they verify "filled in",
while the bug is "consumer didn't read".

#### 1.2 Module boundaries can be bypassed without a trace

`src/frontend/core/typecheck/checker.rs:5618` contains `include!("checker/semantic_tokens.rs");` —
**the only `include!` in the entire repository**. It splices 1,547 lines of text into the `checker`
module. The file **is not declared as any module** (`grep 'mod semantic_tokens'` returns zero hits
repo-wide), its first line is directly `impl TypeChecker {`, it has no own `use` header, and it is
the only file under `checker/`.

Consequences: no module identity, visibility isolation fails, rust-analyzer's go-to-definition and
symbol search break on it, and toolchain analysis counts it as part of `checker.rs`. **The real size
of the `checker` module is 5618 + 1547 ≈ 7,165 lines**, not the 5,618 lines shown by directory
statistics.

#### 1.3 Test wiring relies on human memory; rot happens silently

BFS reachability analysis from the two crate roots `src/lib.rs` / `src/main.rs` shows: **24 files /
2,628 lines never participate in compilation**. Of these, 23 files / 1,081 lines are real orphans.

| Orphan                                      | Scale                | Content                                                                        |
| ------------------------------------------- | -------------------- | ------------------------------------------------------------------------------ |
| Entire `frontend/core/lexer/tests/` subtree | 13 files / 686 lines | 7 shells totaling 19 lines + `mod.rs` 38 lines + **629 lines / 55 real tests** |
| `frontend/pipeline/tests/`                  | 3 files / 14 lines   | All placeholder doc comments                                                   |
| `package/template/tests/`                   | 3 files / 65 lines   | 7 `#[test]`                                                                    |
| `parser/pratt/tests/precedence_inline.rs`   | 95 lines / 6 tests   | Hidden inside a live directory                                                 |
| `typecheck/passes/tests/overload_inline.rs` | 170 lines / 7 tests  | Same as above                                                                  |
| `util/diagnostic/emitter/tests/json.rs`     | 46 lines / 3 tests   | Same as above                                                                  |

`cargo test` cannot find them: a test that never runs will not fail. CI
(`.github/workflows/ci.yml:140`) has no test-count baseline or wiring check either.

**A total of 1,005 lines / 78 tests have never run.** The only lexer-layer file actually running is
`fstring.rs` (wired in via a `#[path]` attribute bypass).

### II. The design capability is sufficient

It must be acknowledged at the same time: this project **already has** mature enforcement
mechanisms; they just haven't been generalized.

The error codes in `src/util/diagnostic/codes/` (137 E codes + 8 W codes = 145) are gated at build
time by `build.rs:19-55` via `tools/code-tables` — parse the registry, validate uniqueness and
segment, compare line-by-line against the RFC-013 markdown code table, **any inconsistency directly
`panic!` and refuse to compile**. RFC-013's document and code therefore stay consistent.

`src/package/` (76 files / 13,012 lines) is the most complete area: **6,227 lines of tests,
accounting for 47.9%**; 5 of 6 `tests/` directories are wired correctly (the only exception is
`template/tests/`, see Section A). Note that this positive example holds at the **directory-level
wiring**, not at the per-file level — of 76 `.rs` files, only 7 contain inline `#[cfg(test)]` and 5
contain `mod tests;`. The 5 RFCs match the code line-by-line, and many "not implemented"
placeholders become explicit errors rather than silent TODOs (cmake in `build/mod.rs:172-174`,
`RegistryDeferred` in `error.rs:74-76`). `.yxpkg` has five layers of safety protection and all of
them are correct.

**The same repository, the same author: `package/` gets test-directory wiring right, while
`lexer/tests/` has never been run once. The difference is not capability, it is whether anyone runs
a check.**

### III. Phantom design

The three documents under `docs/src/dev/design/check/` describe an architecture that **has never
existed**: `CheckSession` (with the Rust code draft in `incremental-checking.md:31-41`),
`ModuleDependencyGraph`, `ModuleCache`, `HotReloader` all have zero hits in the repo; the `traits/`
directory does not exist; the `check_single_module` function does not exist (it only lives in test
comments); even the `command.rs` busy-wait referenced in "Known Limitations" does not exist (that
file has neither `Instant` nor `recv_timeout`).

Cross-file analysis is actually implemented by `src/frontend/module/` (registry 439 + resolver 195 +
roles 388 + orchestrator 1,528, roughly 2,670 lines) — the capability is real, the path is
completely different. **This document writes "designs that might exist in the future" as "defects of
an implemented system".**

`src/frontend/pipeline/tests/compilation_cache.rs` and `incremental_scheduler.rs` (3 lines each) are
the cleanest physical evidence: "test skeletons were built per the design; when no corresponding
implementation was found in the pipeline, they were simply left behind".

In addition, `RFC-018` (LLVM AOT, 1,037 lines, status accepted) has 0 lines of implementation in the
code — the 6 AOT-related hits in `backends/mod.rs` are **all comments**.

## Root Cause Diagnosis

Combining the three categories of symptoms, the root cause is singular:

> **This project treats "design" as a documentation convention, not as an executable constraint.**

| Layer             | Missing enforcement                                               | Existing but ungeneralized exemplar                                           |
| ----------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Stage boundaries  | A single decision point for "which stages this compilation runs"  | —                                                                             |
| Module boundaries | Prohibiting `include!`, limiting `pub(crate)` cross-layer leakage | —                                                                             |
| Cross references  | Detectability of "fields produced but consumed by no one"         | The error-code gate in `build.rs`                                             |
| Test wiring       | A `mod` declaration is required once a directory is declared      | Directory-level wiring convention in `src/package/` (5 of 6 `tests/` correct) |

The position of this RFC is therefore: **transform the three categories of boundaries — stage,
module, and cross-reference — from "documentation conventions" into "facts that can be asserted at
compile time or test time"**, and **establish a single stage decision point in the orchestration
layer**.

## Proposal

### The Four-Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│ L1 编排层 Orchestration                                      │
│   谁决定跑哪些阶段、跑几遍、失败如何传播                      │
│   现状: pipeline.rs(740) + orchestrator.rs(1528)             │
│         + compiler.rs(267) = 5 个入口, 11 处不一致           │
│   目标: 单一 Driver + 可穷举的 Stage 枚举 + Obligations 账本  │
│   详见: ../compiler-architecture/02-stage-contract.md           │
└───────────────────────────┬─────────────────────────────────┘
                            │ 编译单元 (Unit)
┌───────────────────────────▼─────────────────────────────────┐
│ L2 前端层 Frontend                                           │
│   词法、语法、AST 构造                                        │
│   现状: lexer/(3065) + parser/(6341)                        │
│         手工 lexer + 手工 Pratt; 加运算符要改 L2 内          │
│         6-7 处 + 下游 17 个生产文件                          │
│   目标: 词法收敛为唯一实现 + LALRPOP 文法驱动语法            │
│   详见: ../compiler-architecture/05-frontend-paradigm.md        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Module (AST)
┌───────────────────────────▼─────────────────────────────────┐
│ L3 语义与中间表示 Semantic & IR                              │
│   类型检查、静态分析、IR 构造                                 │
│   现状: typecheck/(29466 生产) + ir.rs(905) + ir_gen.rs(8448) │
│         3 套平行类型表示 + 3 套平行运算符枚举;                │
│         ir_gen 单 impl 7952 行 / 116 方法; 无 IR 级校验器    │
│   目标: 类型表示单一化; IR 满足 SSA 构造纪律                  │
│   详见: ../compiler-architecture/03-type-unification.md         │
│         ../compiler-architecture/04-ssa.md                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ ModuleIR
┌───────────────────────────▼─────────────────────────────────┐
│ L4 执行层 Execution                                          │
│   字节码生成、解释器、运行时、标准库                          │
│   现状: codegen/(3436) + bytecode.rs(2422) + executor/(2000)  │
│         + runtime/(1800) + std/(7543)                         │
│   目标: opcode 事实单源化 + 生成期门禁; 执行层不感知 L1-L3    │
│   详见: ../compiler-architecture/06-cleanup-inventory.md        │
└─────────────────────────────────────────────────────────────┘
```

**The four layers are strictly unidirectional**: L1 depends on the interfaces of L2/L3/L4; L2 does
not depend on L3; L3 does not depend on L2 (it only consumes AST data, not calls the parser); L4
only depends on L3's product format.

The known reverse dependencies and parallel definitions list lives in routing table C of
`../compiler-architecture/01-routing.md`.

### Companion Design Documents

Nine of them, stored at `docs/src/dev/architecture/`:

| Document                          | Layer            | Topic                                                                                                                                                                                                                                           |
| --------------------------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `01-routing.md`                   | Cross-layer      | **Feature routing table**, dependency-direction specification, **target directory structure after construction**, future extension guidance                                                                                                     |
| `02-stage-contract.md`            | L1               | Stage contract and obligations ledger: 11 inconsistencies, vulnerability evidence chain, `Stage` / `Obligations` / `Driver` / `ProgramKind` (including `WasmPlayground` on the wasm playground side)                                            |
| `03-type-unification.md`          | L2/L3 root cause | Type-representation unification: per-variant disposition of 26 `ast::Type` variants, 13 forward zero-construction variants, type-table gate                                                                                                     |
| `04-ssa.md`                       | L3               | IR SSA-ification: four categories of defects, SSA form definition, allocator evaluation, 38-item change list                                                                                                                                    |
| `05-frontend-paradigm.md`         | L2               | Lexer convergence to a single implementation / LALRPOP-grammar-driven parsing, change-surface convergence, dead-rung handling, test rebuild                                                                                                     |
| `06-cleanup-inventory.md`         | Whole repo       | Dead code and phantom-design cleanup: reachability method, item-by-item disposition, execution order                                                                                                                                            |
| `07-equivalence-oracle.md`        | Cross-layer      | Equivalence oracles: three-layer oracles, C1–C6 grading, gate design                                                                                                                                                                            |
| **`08-maintenance-mechanism.md`** | **Cross-layer**  | **Repository maintenance mechanism**: three prohibitions (no fabrication / no infinite filling / patches where refactoring is needed), D0–D4 decision procedure, machine-checkable rules, code-review checklist, external convention references |
| **`09-execution-wbs.md`**         | **Cross-volume** | **Multi-level construction task list**: 11 first-level / 45 second-level / 122 third-level tasks, dependencies and parallel grouping, **8 conflict registrations**                                                                              |

`01` and `09` are **long-term reference documents** — the former does not become invalid with any
single refactoring, the latter is the construction checklist. The three prohibitions produced by
`08` constrain every action in P1–P10.

### Equivalence Oracle Grading

**This is the safety net for all refactoring and must be established before any code change.**
Detailed design in `07-equivalence-oracle.md`.

Core principle: **oracles are graded by refactoring category, because different categories require
different strengths of equivalence.** Using a single oracle (usually "IR snapshot identical") to
cover all refactoring is wrong — some refactoring **necessarily changes IR shape**.

| Category | Refactoring content                                                            | Oracle type                                                       | Strength  |
| -------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------- | --------- |
| **C1**   | Pure relocation (splitting files, changing directories, extracting submodules) | IR normalized snapshot **zero-diff**                              | Strongest |
| **C2**   | Orchestration change (staging, unified Driver)                                 | Each entry's diagnostic set identical + corpus behavior identical | Strong    |
| **C3**   | Type-representation convergence                                                | Diagnostic **codes** identical (message wording may change)       | Strong    |
| **C4**   | IR shape change (SSA-ification)                                                | **Behavioral equivalence** + IR structural invariants             | Medium    |
| **C5**   | Frontend-paradigm change                                                       | AST snapshot + diagnostics + behavior, all three used             | Strong    |
| **C6**   | Pure deletion                                                                  | No equivalence needed, only confirm no references                 | —         |

**The distinction between C1 and C4 is key**: C1 requires the IR to be byte-identical; C4
acknowledges that the IR will change and uses "identical program behavior + IR satisfies invariants"
instead. Forcing C4 to use snapshots will tempt the team to relax the oracle.

Three-layer oracle:

1. **IR static validator** — dominant use-before-def, jump targets exist, type consistency,
   inner-layer isolation. Far stronger than snapshots, covering defects that snapshots cannot
   detect, like "IR self-consistent but wrong value". Must **first run green on the existing
   (non-SSA) IR**.
2. **Normalized IR snapshot** — strip `Span`, rename temporaries by occurrence order, relativize
   global slots, sort predecessors. Check in, manually review the diff. **Known limitation**:
   normalization erases "which register was used by the Nth argument", so **alone it does not cover
   `arg_regs` semantic reshuffling**.
3. **End-to-end corpus diff** — diagnostic list, exit code, stdout/stderr compared item by item.

> **⚠️ Structural limitation of corpus coverage**: there is **no `yaoxiang.toml`** under `tests/`
> (measured: 0), so all **293** `.yx` corpora in `tests/yaoxiang/` go through the single-file path
> (`check_files_with_diagnostics`'s standalone branch → `check_single_file` → `Pipeline::run`).
>
> **Consequence**: the third layer oracle **can only verify single-file-path changes**. For the
> multi-file side (the four entry points of `orchestrator`) and the multi-file behavior of
> types/SSA, **there is currently no corpus coverage at all**. P4 "unified Driver" is the single
> biggest risk point of the plan, and **the multi-file corpus layer must unconditionally be
> established in P2** (Resolution D40) — it is P4's only executable source of behavioral oracles.

## Implementation Strategy

### Stage Sequence

```
P0  仓库维护机制与代码放置规程 08        ← 先立规矩:三条禁令 + D0-D4 决策程序
P1  复活孤儿测试并修缺陷        06 §S1   ← 最便宜,收益最大
P2  建立等价性判据基线          07 全部
P3  修正确性漏洞(最小方案)      02 §修复
P4  阶段契约与统一 Driver        02
P5  checker 文件内拆分          09 §P5 补齐
P6  类型表示单一化              03
P7  中间表示 SSA 化             04     } 可并行
P8  前端范式变更                05     }
P9  防反弹门禁(统一脚本清单)     01 + 08
P10 其余清理与状态修正          06 §S2-S6
```

**Core serial constraints (not interchangeable):**

| Constraint   | Reason                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0 → P1      | The three prohibitions constrain every action in P1 — 1.2.x is exactly adding `mod` declarations to `mod.rs`, which is what produced the `precedence_inline.rs` life-and-death conflict                                                                                                                                                                                                                   |
| P1 → P2      | Reviving tests will change test counts and the corpus baseline. Building the baseline first will become invalid after P1                                                                                                                                                                                                                                                                                  |
| P2 → P3      | The vulnerability oracle must be **written red first** to prove the fix effective. **Narrowing (D51)**: P3's prerequisites are only 2.3.2 (multi-file corpus layer) + 2.4.1/2.4.3 (two red oracles); the rest of P2 can run in parallel with P3, but P4 must wait for P2 to fully complete — the bleeding stop on the correctness vulnerability must not be blocked by oracle-infrastructure construction |
| P3 → P4      | Fix the bug first, then refactor. Reversing the order will let the bug be cemented as "established behavior" by the stage table                                                                                                                                                                                                                                                                           |
| P4 → P5      | P5 starts with `include!` → real `mod` (09 §P5 5.1), the subsequent checker split moves the same file and must be done continuously after P4                                                                                                                                                                                                                                                              |
| P5 → P6      | Consecutive changes to the same file must be separated, otherwise regressions cannot be bisected                                                                                                                                                                                                                                                                                                          |
| P6 → P7 / P8 | Until type representation is converged, the new SSA IR will grow into a third representation                                                                                                                                                                                                                                                                                                              |
| P7/P8 → P9   | The initial baseline of each gate script (e.g. `pub(crate)` leakage count) only takes its final value after P7/P8                                                                                                                                                                                                                                                                                         |
| P7 ∥ P8      | Disjoint file sets, can run in parallel                                                                                                                                                                                                                                                                                                                                                                   |

**Why P0 is a separate stage**: P9's CI scripts are **after-the-fact checks** that can only catch
already-written bad code; "refactoring where a patch was needed instead" is a **before-the-fact
judgment** that only the D0–D4 decision procedure can intercept. The two cannot substitute for each
other.

### Stage Highlights

| Stage   | Scope                                                                                                                                                                                                  | Acceptance                                                                                                                                                                                                                              | Rollback point                                                                                                   |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **P0**  | `CONTRIBUTING.md` procedure section + 3 new gate scripts (`check-concepts` / `check-fanout` / `check-boundary`); **no compiler source change, no line-count gate**                                     | `check-concepts.py` **must report 3 parallel operator enums and 2 parallel type representations (the `ir::Type` alias counted in) on un-modified code**; intentionally exceeding limits / adding a 6th entry-style wiring must turn red | Pure addition; deleting the script is enough                                                                     |
| **P1**  | 5 files add `mod` declarations; **1,005 lines / 78 tests revived**                                                                                                                                     | Test count goes up; **real defects expected to surface** (the `literals.rs` overflow path, the never-tested `\x` / `\u` illegal escapes)                                                                                                | Revert file by file; **fixed defects should not be rolled back**                                                 |
| **P2**  | New `verify.rs`, snapshot baseline, corpus-diff framework, **multi-file corpus layer**, performance baseline (criterion smoke benchmark), `scripts/ci/check-*`                                         | `verify_loose` runs green; vulnerability oracles **written red**                                                                                                                                                                        | Oracle code can be removed in its entirety                                                                       |
| **P3**  | `types.rs` (`proof_calls` made private), three consumer sites in `orchestrator.rs`                                                                                                                     | Vulnerability oracles **turn green**; **intentionally removing the fix must turn it red again**                                                                                                                                         | Pure behavior correction                                                                                         |
| **P4**  | New `src/driver/`; rewrite 5 entry points; **4.5: `ReleasePlan` / `overload_resolutions` keys `Span` → `PlanId` (D20)**                                                                                | Identical diagnostic set + identical corpus behavior (C2); span-keyed silent-failure count to zero                                                                                                                                      | **Largest risk point**; keep old entry points, Driver unwired is enough to roll back                             |
| **P5**  | `include!` → real `mod` (5.1); split `checker.rs` into `refinement` / `annotations`                                                                                                                    | **IR snapshot zero-diff** (C1); `pub(crate)` path in `statements.rs:16` unchanged                                                                                                                                                       | Revert file by file                                                                                              |
| **P6**  | `ast.rs`, `types/mono.rs`, `solver.rs`, `ir.rs:3`, `bytecode.rs:2353-2390`; stage closeout executes directory rename (D1: `typecheck/`→`sema/`, `middle/core/`→`middle/ir/`)                           | Diagnostic **codes** identical (C3); rename batch is pure relocation (C1 zero-diff)                                                                                                                                                     | Stage-by-stage commits, variant disposition and synonym-table deletion separated; rename gets a dedicated commit |
| **P7**  | `ir.rs`, `ir_gen.rs` (8,448 lines), `bytecode.rs`, `translator.rs`; **7e: fix the three `.42` data-loss points (`upvalue_count` / `exception_handlers` / `globals`) + `VERSION` 4→5 (D17, D52)**       | **Behavioral equivalence + `verify_ssa` green** (C4). **Not IR snapshot equality**; `.42` round-trip test                                                                                                                               | Revert batch by batch                                                                                            |
| **P8**  | `lexer/*`, `parser/*`                                                                                                                                                                                  | AST snapshot + diagnostics + behavior (C5)                                                                                                                                                                                              | Revert file by file                                                                                              |
| **P9**  | New `scripts/ci/check-*.py` total of 10 (sole list in [09](../../dev/architecture/09-execution-wbs.md) §P9: 3 with P0, 1 with P1, 3 with P2, 2 with P4, 1 with P7 introduced; P9 collects and hardens) | **Intentionally leaving an obligation without a consumer must turn red**; **intentionally creating an orphan test directory must turn red**; **intentionally adding `include!` must turn red**                                          | Delete scripts                                                                                                   |
| **P10** | Documentation disposition, RFC status correction, `pub` items visibility downgrade, opcode decisions                                                                                                   | `check_tracking.py` passes; doc site has no 404                                                                                                                                                                                         | Independent PR                                                                                                   |

### Definition of Done per Stage

A stage is considered complete only when all the following are met:

1. All "Implementation Strategy" steps in the owning document are committed
2. The oracle for the category (C1–C6) in `07-equivalence-oracle.md` is **green in CI**
3. `cargo test` is fully green, and **the test count is no lower than at the start of the stage**
   (preventing "delete tests to turn green")
4. `cargo clippy --all --all-features -- -D warnings` is clean
5. `python scripts/rfc/check_tracking.py` exit code 0
6. The stage has an independent revert unit — **if a stage cannot be reverted alone, it is too large
   and needs to be split again**
7. For stages touching the compilation pipeline or execution path (P4 / P6 / P7 / P8), the
   performance baseline comparison in 07 shows no unexplained >10% regression

### Global Acceptance Gates

After all stages are complete, the following must be true:

| #   | Condition                                           | Verification                                                                       |
| --- | --------------------------------------------------- | ---------------------------------------------------------------------------------- |
| G1  | Single decision point for compilation stages        | Answering "which stages ran, which didn't, why" only requires looking at one place |
| G2  | No "produced but unconsumed" fields                 | `check-obligations.py` green                                                       |
| G3  | `include!` count is 0                               | `grep -rn 'include!' src/`                                                         |
| G4  | No orphan tests                                     | `check-test-wiring.py` green                                                       |
| G5  | No reverse dependencies                             | `check-module-boundary.py` green                                                   |
| G6  | Type representation is unique                       | Parallel representations converged; hand-written synonym tables disappeared        |
| G7  | parser contains no type/predicate hardcoding        | Literals like `"Terminates"` replaced by data flow                                 |
| G8  | Adding a binary operator changes ≤ 3 places         | Manual review + routing table update                                               |
| G9  | `layers/README.md` describes the actual layer order | Doc consistent with `check_module_impl`                                            |
| G10 | `TRACKING.md` has an "Implementation Status" column | Implementation status of 52 RFCs is queryable                                      |

## Trade-offs

### Advantages

- **Gives all subsequent changes a landing point.** "Adding an operator changes a dozen places, and
  missing one no one knows" cannot be improved by memory, only by a routing table + gates.
- **Single root cause.** Three categories of symptoms (stage inconsistency, `include!`, test-wiring
  breakage) all stem from "boundaries are conventions, not constraints", and can be solved in one go
  with the same class of mechanism.
- **Not blocking.** This RFC has zero code changes. The four-layer model is descriptive; after
  adoption, only parts of it may still be executed.
- **Oracles first.** P2 is established before any code change, and the graded design avoids the
  common failure of "snapshot too strict → tempting to relax".

### Disadvantages and Risks

- **The serial chain is long.** P1 → P2 → P3 → P4 → P5 → P6 → P7/P8 is almost entirely serial, and
  later stages must wait for earlier ones to complete.
- **P1 will introduce new bug-work**, and it is the easiest step to skip because of "needing to fix
  bugs, so we'll do it later" — but skipping it means all subsequent stages' acceptance oracles are
  built on a false coverage.
- **P4 is the single biggest risk**: a single change touching 5 entry points.
- **P6 has the widest impact**: touching parser, formatter, spawn, and orchestrator's common
  dependency.
- **P7's line count will net increase by 900–1,600 lines, not decrease.** Splitting introduces
  boilerplate, `verify` 300–600 lines, register allocator 600–1,000 lines. **If "shorter code" is
  the success criterion at project launch, this stage will be judged a failure** — this must be
  aligned at project launch.

## Alternatives

### A. Only split files, no paradigm change

Split 8,448 lines into 10 files of 800 lines each.

**Not adopted.** Splitting does not eliminate any root cause: 6 hand-written save/restore sites are
still scattered across 10 files; the `arg_regs` semantic reshuffle in `generate_call_expr_ir` still
requires manual reasoning; "adding an operator changes a dozen places" is still a dozen places.
Splitting improves navigation, not correctness.

### B. Only add gates, no structural change (including line-count ratchet)

**Not adopted as the sole solution, but it is part of this RFC.** Gates can prevent regression, but
cannot fix the present — `ir_gen.rs` already has 8,448 lines, and gates can only freeze it at that
number.

### C. Start over from scratch with a new compiler skeleton

**Not adopted.** `src/package/` proves that this team can get complex systems right. The problem is
not capability. Moreover, the existing 293 `.yx` corpora provide a ready-made source of equivalence
oracles; rewriting from scratch would waste this asset.

### D. Use an off-the-shelf compiler framework (e.g. LLVM as the sole backend)

**Not adopted.** The `Executor` trait (`backends/mod.rs:356`) already reserves an abstraction point,
but has only one implementer. Introducing LLVM falls under `RFC-018` (1,037 lines of design, 0 lines
of code) and should be **decided independently** — it changes "compile to what", not "how to
organize the code".

## Resolution Register: All Open Items Decided

> **This section is the sole authority.** All original `- [ ]` open questions and "Conflict
> Registrations" in each companion document **are converged into this table**, and are no longer
> individually retained. **No "TBD", no "deferred", no "optional".** Each entry gives a decision and
> a reason. If the technical premise of some decision is found to be invalid during implementation,
> **the correct action is to return to this table, change the decision, and explain why**, not to
> bypass it.

### Directories and Naming

| #   | Topic                                                                                                                                                                                   | **Decision**                                                                                                                                             | Reason                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Whether directory rename (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`, move `parser/ast.rs` to top-level `src/ast/`, move opcode vocabulary to `middle/bytecode/`) is worth doing | **Do it.** No staging, no "optional"; complete alongside P5/P6 (top-level AST field and bytecode field merge in the same batch, both C1 pure relocation) | Boundaries are guaranteed by CI, but **the directory name is the first-glance signal of responsibility**. `middle/core/` holds `ir.rs` + `ir_gen.rs` + `bytecode.rs` — three things; the name has already failed; AST is consumed by all layers and placing it under `frontend/` would mislead downstream paths (6 of 7 industry examples place it at the top level alongside the parser) |
| D2  | Naming convention                                                                                                                                                                       | Keep `type_.rs` / `fn_.rs` (avoid Rust 2024 reserved words); prohibit `xxx_v2` / `xxx_new` directories                                                   | Parallel copies are a common source of dead code (the `Precedence` enum in `pratt/precedence.rs` is parallel to the actually-effective BP constants, with zero production references; `Expr::FnDef` is a parallel construction path that never executes)                                                                                                                                  |

### Stage Contract (02)

| #   | Topic                                                                                                                                                                                                                                      | **Decision**                                                                                                                                              | Reason                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D3  | `check_module` (`typecheck/mod.rs:81`, fail-fast; early-exit implementation in `inference/statements.rs:2797-2799`) vs `check_module_collect_all` (`typecheck/mod.rs:93`, collect-all; call sites `orchestrator.rs:124` / `:493`) — merge? | **Do not merge.** Keep both APIs, driven by `Program`'s aggregation mode field (`Aggregation: FailFast \| CollectAll`, see 02)                            | The two semantics are indeed different (`CollectAll` serves LSP). Forcing a merge would introduce `Option` noise                                                                                               |
| D4  | Which of the 16 fields of `Obligations` "must be consumed"                                                                                                                                                                                 | **All 16 enter the ledger.** Fields with no consumer (e.g. `module_namespaces`) are deleted in P4's S4, **leaving no orphan fields**                      | The point of the ledger is no exceptions                                                                                                                                                                       |
| D5  | Diagnostic level of `assert_drained()`                                                                                                                                                                                                     | **Final: E-level** (error); in P4, first come up at W-level for observation, then upgrade to E at the end of the same stage (the two-step walk in 02 §S4) | Staying at W long-term equals no gate — W does not change exit code; but upgrading to E in one step would make the C2 oracle unusable, so E is the terminal state and W is just a transitional state within P4 |
| D6  | `ownership.rs:627` creates a new Z3Backend every time                                                                                                                                                                                      | **Make it a singleton**, unified with the global `LazyLock` at `predicate.rs:34-36` into a `SolverProvider`                                               | Three acquisition strategies and two failure philosophies (one panics, one is silent) must converge                                                                                                            |
| D7  | `layers/README.md` layer order is the reverse of the actual                                                                                                                                                                                | **Fix it**, fall into 4.4.1 of P4                                                                                                                         | README describes an unimplemented intent-architecture; fixing it will expose the previously silent `Unproven` — **that is what should be exposed**                                                             |

### Type Representation (03)

| #   | Topic                                                                                                                                                                                     | **Decision**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D8  | `AssocType` (`ast.rs:463-471`) — delete?                                                                                                                                                  | **Delete.** Zero production constructions (the only construction in the entire repo is in test `types/tests/mono.rs:178`, which does not change the conclusion); re-verified by the T1 gate as a standalone                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Disposed of in the same way as the other 11 zero-construction variants; if the associated-type syntax is enabled in the future, a new proposal redefines it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| D9  | T1 gate uses `syn` or Python                                                                                                                                                              | **Use `syn`.** `tools/code-tables` is already a Rust crate, reuse it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Building another Python parser is re-implementation, violating Prohibition One                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| D10 | `Type::Void` fallback (`parser/statements/types.rs:836`) — change to what                                                                                                                 | **Change to `Err(Diagnostic)` return**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Silent fallback is a form of "fabrication" — covering up absence with a fake value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| D11 | `NameKind` judgment + LSP path probe                                                                                                                                                      | **LSP reuses `TypeEnvProbe`**, no separate probe channel                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | `lsp/world.rs` already holds `SemanticDB`, low reuse cost                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| D12 | `const_data::BinOp` / `ast::BinOp` — who generates whom                                                                                                                                   | **Use `ast::BinOp` as the authority**, the `const_data` side aligns by semantics (no forced `Neq` / `Ne` rename)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | AST is the primary representation; renaming ripples to 12 reference sites with no benefit                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| D13 | `classify_generic_params` uses `signature_params` or `Lambda.params`; same in `ir_gen.rs:1380`                                                                                            | **Not an open question, but a mandatory verification item for stage 6.1.3 of P6**; conclusion written into the PR                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | This is an execution step, not a deferred strategy                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| D14 | Statistics of non-canonical type names among the 293 corpora                                                                                                                              | **Same as above: a task** (6.1.1), not an open question                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Same as above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| D15 | Classification of stage 2 "behavior-preserving" vs "behavior-changing"                                                                                                                    | **Classify by the C3 oracle: design changes go in independent commits, wording changes do not count**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Already defined in 07                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| D54 | Fn annotation-vs-body alignment missing (check green, runtime type error)                                                                                                                 | **Annotation-driven checking mode.** Bound-periodic parameter types drive the lambda head bit-by-bit, exit (tail expression and return) uniformly check against annotation return types, registration type = annotation form; parser name merging, #295 append, `value_params` bit-by-bit fill count precondition retired. Representation prerequisite: `Type::Fn.params: Vec<Param>` (names and types co-located), lands as new 6.8 in P6                                                                                                                                                                                                                                                                                                                                       | The alignment mechanism is currently three disconnected fragments (parser merge / registration annotation form / body check forks by syntactic form), and the invariant "declaration = implementation" has no owner — `f: () -> Int = () => "hello"` passes while `f: () -> Int = { "hello" }` errors (same semantics, different syntax), check green, runtime E6007; this is the same-pattern onset of the proof_calls silent channel (§1.1) in the annotation subsystem. The alignment-enabled batch is a behavior fix: new diagnostics, baseline update, C3 not applicable                                                                                                                                  |
| D55 | LSP semantic index (SemanticDB definitions/references) production-side missing: go-to-definition, hover, find-references, rename all silently fail; multi-file semantic coloring collides | **Production side goes to checker.** definitions/references/imports are registered by checker/inference at the name-resolution convergence point (definition side at `add_var`/`add_param` and module-level signature collection sites, reference side at `get_var_info` resolution sites, carrying `resolves_to`); LSP only consumes, no second set of name resolution is built; World session library upserts by file (std and built-in types are resident), not wholly replaced by single-file check results; go-to-definition precise-only, missing data returns empty, no name-based fallback. The single-file internal implementation goes in an independent issue; the in-project file and diagnostic pipeline (orchestrator) lands as new 4.6 in P4, prerequisites 4.2.1 | All 6 consumer-side handlers are in place and tests are green (hand-crafted data), production side has zero write sites — `add_reference` has had no production call site since its introduction; the go-to-definition rewrite commit removed name-based fallback and `SymbolIndex` without building the precise-resolution data source, so go-to-definition died the day of the rewrite (`resolve_reference` reads a references table no one writes). Silent channel (§1.1) third instance (after proof_calls, D54 annotation alignment); World wholesale replacement also makes the std/built-in symbols loaded at session start evaporate on first didOpen. The probe-reuse criterion from D11 carries over |

### Intermediate Representation (04)

| #   | Topic                                                        | **Decision**                                                                                                                                                                           | Reason                                                                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D16 | `Instruction::Phi` static selection                          | **Plan A: IR-only, `codegen` expands to Move sequence**                                                                                                                                | No new `PHI` opcode, avoiding `.42` format expansion                                                                                                                                                                                                                                     |
| D17 | `bytecode.rs:2312` `upvalue_count: 0`                        | **Fix; when adding the field bump the `.42` version from 4 to 5** (format header already has `MAGIC` + `VERSION: u32` field, `codegen/bytecode.rs:14-16`, read side checks by version) | This is a data-loss defect, not dead code; after the version bump, old `.42` is rejected by the read side per existing behavior (`.42` is a build artifact, cross-version compatibility is not the goal); already-allocated unused opcode values are handled along with the version bump |
| D18 | After deletion, is the u8/255 slot ceiling enough?           | **Must be measured at the end of batch d**; if not, expand to `u16`                                                                                                                    | This is a verification item, not a deferred item                                                                                                                                                                                                                                         |
| D19 | Linear-scan allocator — should a prototype be timed first?   | **Do not prototype, implement directly.** Time it under CI                                                                                                                             | Timing does not change the design decision, it is procrastination                                                                                                                                                                                                                        |
| D20 | `ReleasePlan` / `overload_resolutions` Span keying           | **Change to `PlanId`**, eliminate cross-layer span contracts; fall into P4                                                                                                             | Span mismatch causes Drop to silently lose, one of the most insidious classes of defects                                                                                                                                                                                                 |
| D21 | `method_def_ordinals` to 03 or 04                            | **To 04** (P7 batch c)                                                                                                                                                                 | It is IR-construction state, not a type issue                                                                                                                                                                                                                                            |
| D22 | Whether the `synth.rs` boundary generalizes to the entire L3 | **Generalize to the entire L3**; L4 included together after a separate check                                                                                                           | Boundary rules are either complete or invalid                                                                                                                                                                                                                                            |
| D23 | `compile_pattern` / `eval_const_expr` — "do not handle"?     | **Included in batch d scope**, not exempt                                                                                                                                              | Avoiding work by labeling it "unhandled" is patch-thinking                                                                                                                                                                                                                               |

### Frontend (05)

| #   | Topic                                                                                          | **Decision**                                                                                                                                                                                                                                                                                                               | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D24 | Can LALRPOP fully reproduce the existing diagnostics?                                          | **Must reproduce.** C5 is not relaxed. Use explicit error productions to write the `synchronize()` synchronization set and skip timing into the grammar                                                                                                                                                                    | Not being able to is an implementation defect, **not a reason to relax the oracle**; honestly report and reassess (including "keep Pratt"), not change the oracle                                                                                                                                                                                                                                                                                                                                |
| D25 | F-string span determined at lex time will change, is that acceptable?                          | **Not acceptable.** Span must be recorded as **absolute offset** at lex time, diff comparing `span.file/line`                                                                                                                                                                                                              | Span changes affect diagnostic location and are not acceptable                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| D26 | Does associativity need to be declared explicitly?                                             | **Yes.** Explicitly declare left/right associativity in the grammar, do not rely on the `bp_right = bp_left + 1` convention                                                                                                                                                                                                | The current convention is the source of the magic numbers like `BP_RANGE` hardcoded to `(6,7)`                                                                                                                                                                                                                                                                                                                                                                                                   |
| D27 | `is_old_function_syntax` (36 lines) — disposition                                              | **Option 1: Delete naturally with the grammar migration.** `f(Int) -> Int = ...` cannot match any production                                                                                                                                                                                                               | Specifically probing removed syntax is an anti-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| D28 | In stage 0, delete `pub use precedence::*;` in `pratt/mod.rs:11`                               | **Delete**                                                                                                                                                                                                                                                                                                                 | It is the exposure surface of the 96-line dead rung                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| D29 | 8 responsibility segments of `parse_assign_after_target`                                       | **All split, none retained.** `apply_semantic_side_effects` is handed off to P6 but **not deleted**                                                                                                                                                                                                                        | Splitting is re-dividing responsibility, not deleting it                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| D30 | Full-corpus time cost in stages 2-4                                                            | **Measured and recorded in the P2 baseline stage**                                                                                                                                                                                                                                                                         | This is a verification item, not a deferred item                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| D53 | What does a bare identifier in a parameter position (the `Int` in `(Int, Int) -> Int`) read as | **Unnamed typed parameter.** checker resolves in the type namespace; failure to resolve reports E; the unnamed signature requires the lambda head to carry its own parameter names (per RFC-007 shorthand rules); RFC-010 form table and interface examples (e.g. `(Surface)`) reconciled together. Lands as new 8.9 in P8 | parser currently reads it as "inferred parameter name" (no-colon branch in `parse_fn_type_with_names`), annotation is zero-constrained and violates RFC-007 §25 "omitting both sides will be rejected"; the std interface layer (e.g. `ok: (T) -> Result(T, E)`) and dozens of corpora empirically reading the bare form as unnamed typed parameters means migration cost is zero; forcing `name: type` (reject) would migrate the entire std layer and forbid the most intuitive way of writing |

### Cleanup and Oracles (06 / 07 / 08)

| #   | Topic                                                         | **Decision**                                                                                                                                                                                                                                                              | Reason                                                                                                              |
| --- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| D31 | Which PR fixes the defects exposed by S1 reviving tests       | **The same PR**, test revival and defect fix together                                                                                                                                                                                                                     | Splitting them will tempt future people to "fix it next time"                                                       |
| D32 | `Switch` opcode (interpreter has implementation, no producer) | **Delete the opcode and interpreter implementation.** Handled together with the `.42` version bump (D17)                                                                                                                                                                  | An opcode with no producer is a dead path; the whitelist only hides it                                              |
| D33 | `UnaryOp::Not` silently degraded to `I64_NEG`                 | **Fix.** `opcode()` must distinguish the `op` field, not ignore it                                                                                                                                                                                                        | Silently changing semantics is the most dangerous class                                                             |
| D34 | `Instruction::TailCall` encoding branch                       | **Delete** (no construction site in the entire repo)                                                                                                                                                                                                                      | Same reasoning as D32                                                                                               |
| D35 | wasm 27-file cfg branches                                     | **Delete unreachable parts in the playground scenario, keep reachable ones.** Falls into P10                                                                                                                                                                              | wasm target is already built (independent shim crate), the branches are load-bearing, only delete unreachable parts |
| D36 | `TRACKING.md` add an "Implementation Status" column           | **Add.** `check_tracking.py` gains generation logic                                                                                                                                                                                                                       | Currently the 26 accepted RFCs do not record implementation status at all; RFC-018 rotted this way                  |
| D37 | § Dead reference to `docs/superpowers/` (gitignored document) | **Delete the comment**                                                                                                                                                                                                                                                    | Pointing to a gitignored planning document is a dead reference                                                      |
| D38 | If `verify_loose` doesn't run green                           | **No exemption.** If it doesn't run green, `ir_gen` has an implicit "same-slot multi-write" dependency, **which is a defect that must be fixed first**                                                                                                                    | The exemption list is new technical debt, equivalent to using gates to hide design problems                         |
| D39 | Snapshot baseline volume and compression                      | **Do not introduce git-lfs, accept the volume**                                                                                                                                                                                                                           | Snapshots are regression oracles, readability over volume                                                           |
| D40 | Multi-file corpus layer                                       | **Unconditionally required** (P2's 2.3.2), not "if not done, P4 pauses"                                                                                                                                                                                                   | The conditional clause is leaving a back door for oneself                                                           |
| D41 | Whitelist for `test_release_plan_spans_consumed`              | **No whitelist, the diff set must be empty** (this test is new in P2; the existing `WHITELIST = ["SWITCH"]` in the repo belongs to the opcode round-trip test and is unrelated). If it cannot be empty, the ReleasePlan contract has a defect; fix the contract (see D20) | A whitelist legalizes bugs                                                                                          |
| D42 | Does `verify()` hook into `cargo test` or CI                  | **CI**                                                                                                                                                                                                                                                                    | Full-corpus verification time is unsuitable for every local run                                                     |
| D43 | Similarity algorithm for Prohibition One's criterion A        | **Jaccard ≥ 0.5**                                                                                                                                                                                                                                                         | Already fixed in `08`'s main text, open question removed                                                            |
| D44 | Does a PR have to declare responsibility attribution          | **Yes.** PR template adds a required field                                                                                                                                                                                                                                | Responsibility judgment is not machine-checkable, can only be enforced by process                                   |
| D45 | Who reviews the `// reason:` exemption                        | **Merged into the PR review checklist**, no dedicated person                                                                                                                                                                                                              | Without team division, designating a dedicated person equals designating no one                                     |

### Project and Process

| #   | Topic                                                                  | **Decision**                                                                                                                                                                                                         | Reason                                                                                                                                                                                                                                                                                          |
| --- | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D46 | `RFC-018` (accepted, 0 lines of code)                                  | **Move back to `draft/`**                                                                                                                                                                                            | Per `rfc/index.md:131`'s definition "accepted = enters implementation", it has not                                                                                                                                                                                                              |
| D47 | Stage-to-Issue mapping                                                 | **Open one Issue per first-level stage**, `check_tracking.py`'s `issues_impl` registry                                                                                                                               |                                                                                                                                                                                                                                                                                                 |
| D48 | Multi-file corpus layer location                                       | **`tests/yaoxiang-multifile/`** (new)                                                                                                                                                                                | Extending `multifile.rs` would mix semantic tests and contract corpora                                                                                                                                                                                                                          |
| D49 | Doc-site navigation                                                    | **Add.** A new section next to "Tooling Design" at `config.js:255`                                                                                                                                                   | Without it, the nine companion documents are unreachable                                                                                                                                                                                                                                        |
| D50 | Stage parallelism (multi-file typecheck)                               | **Not in this round.** Becomes an independent topic after oracle stability                                                                                                                                           | Parallelism would mask order-dependence defects, conflicting with this round's investigation goal                                                                                                                                                                                               |
| D51 | P3 bleeding-stop channel                                               | **Allowed.** P3 prerequisites narrow to 2.3.2 + 2.4.1 + 2.4.3; the rest of P2 (IR validator / snapshot / single-file diff / performance baseline) can run in parallel with P3; P4 must wait for P2 to fully complete | The bleeding stop on the correctness vulnerability (`Sorted(3)` silently passing) must not be blocked by snapshot-infrastructure construction; P4 requires all three oracle layers in place, so it is not relaxed                                                                               |
| D52 | `.42` three hard-coded discards (`bytecode.rs:2312` / `2315` / `2341`) | **All rolled into P7 (7e), no "independent issue".** `2315` (exception table) and `2341` (globals) were originally planned as independent issues, now consolidated                                                   | The exception-table loss makes throw/try behavior wrong when `.42` is run directly — throw/try are core language semantics; this refactoring leaves no unimplemented remnants of core functionality. The three fix together needs only one `VERSION` bump (4→5), cheaper than two version bumps |

### Sole Remaining Data Item

| #   | Item                | Description                                                                                                                               |
| --- | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| —   | RFC's `issue` field | Already filled in: [#430](https://github.com/ChenXu233/YaoXiang/issues/430) (2026-10-05, tracking issue created on the day of acceptance) |

## Appendix: Glossary

| Term                         | Definition                                                                                                                                             |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage**                    | A step in a single compilation that is exhaustively enumerable. The set of stages must be an exhaustively enumerable enum, not registerable at runtime |
| **Orchestration Layer (L1)** | The layer that decides which stages run, how many times, and how failures propagate                                                                    |
| **Frontend Layer (L2)**      | Lexing and parsing, producing AST                                                                                                                      |
| **IR Layer (L3)**            | Type checking, static analysis, IR construction                                                                                                        |
| **Execution Layer (L4)**     | Bytecode generation, interpreter, runtime, standard library                                                                                            |
| **Obligation**               | A contract item produced by a stage that must be consumed downstream or the compilation fails                                                          |
| **Obligations Ledger**       | The container of all obligations, settled together at the tail of the stage table                                                                      |
| **Orphan**                   | A file that is never referenced by any `mod` declaration and therefore never participates in compilation                                               |
| **Shell**                    | A file that has been declared and entered the compilation product, but has only doc comments or unused `use`s, with zero assertions                    |
| **Equivalence Oracle**       | Executable checks proving behavior is unchanged before and after refactoring                                                                           |
| **C1–C6**                    | The refactoring category grading that determines oracle strength. **There is no C5′**                                                                  |
| **Separation of Concerns**   | The only solution to scale problems: one module carries only one class of responsibility. **No line-count / volume gates are set**                     |
| **Resolution Register**      | The "Resolution Register" section of this RFC, the final ruling on all open items                                                                      |

## Appendix: Design Decision Records

| Decision                              | Decision                                                                                                                                                                                                                          | Date       | Recorder  |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------- |
| Layering granularity                  | Four layers (L1–L4)                                                                                                                                                                                                               | 2026-10-03 | ChenXu233 |
| Document structure                    | **1 RFC + 9 companion design documents** (`docs/src/dev/architecture/`, at the same level as `check/`, `formatter/`)                                                                                                              | 2026-10-03 | ChenXu233 |
| Directory rename                      | **Do it** (D1), no staging                                                                                                                                                                                                        | 2026-10-03 | ChenXu233 |
| AST location                          | **Top-level `src/ast/`**, at the same level as the parser (industry majority; `check-boundary` no-waiver clause)                                                                                                                  | 2026-10-04 | ChenXu233 |
| Opcode location                       | **`middle/bytecode/opcode.rs`**, merged with the bytecode field (vocabulary and format share a field, dependency direction restored to L4→L3)                                                                                     | 2026-10-04 | ChenXu233 |
| Scale gates                           | **All cancelled.** Scale is solved by separation of concerns, adopting Go's official position                                                                                                                                     | 2026-10-03 | ChenXu233 |
| Grammar paradigm                      | **Full LALRPOP grammar-driven**; rejecting the intermediate state of "keep Pratt with table lookups"                                                                                                                              | 2026-10-03 | ChenXu233 |
| Equivalence oracles                   | **C1–C6 six-category grading, no relaxations** (there is no C5′)                                                                                                                                                                  | 2026-10-03 | ChenXu233 |
| Deletion policy                       | Full deletion is allowed; orphan tests with real assertions are **revived, not deleted**                                                                                                                                          | 2026-10-03 | ChenXu233 |
| Verification method                   | This set of documents does not run cargo; static evidence + line numbers are authoritative                                                                                                                                        | 2026-10-03 | ChenXu233 |
| Stage order                           | P0 maintenance mechanism → P1 revive tests → P2 oracles → … → P10 cleanup                                                                                                                                                         | 2026-10-03 | ChenXu233 |
| Open items                            | **All 50 nailed down** (Resolution Register D1–D50), no trade-offs left                                                                                                                                                           | 2026-10-03 | ChenXu233 |
| P3 bleeding-stop channel              | **Allowed** (D51), prerequisites narrow to 2.3.2 + 2.4.1 + 2.4.3                                                                                                                                                                  | 2026-10-05 | ChenXu233 |
| `.42` three data-loss points          | **All rolled into P7 (7e), no independent issue** (D52)                                                                                                                                                                           | 2026-10-05 | ChenXu233 |
| Bare identifier in parameter position | **Unnamed typed parameter** (D53), construction-time rejection falls on "type not resolvable"; falls into P8                                                                                                                      | 2026-10-05 | ChenXu233 |
| Fn annotation alignment               | **Annotation-driven checking mode** (D54), representation prerequisite `Type::Fn.params: Vec<Param>`; falls into P6                                                                                                               | 2026-10-05 | ChenXu233 |
| LSP semantic-index production side    | **Checker convergence-point registration + LSP consumes only + World upsert by file + precise-only** (D55); single-file internal implementation goes in an independent issue, project pipeline unification lands as new 4.6 in P4 | 2026-10-06 | ChenXu233 |

## References

- [RFC-010 Unified Type Syntax](../accepted/010-unified-type-syntax.md) — the direct source of
  type-representation convergence
- [RFC-011 Generic Type System](../accepted/011-generic-type-system.md)
- [RFC-011a Interface Implementation](../accepted/011a-interface-implementation.md)
- [RFC-013 Error Code Specification](../accepted/013-error-code-specification.md) — exemplar of
  build-time gates
- [RFC-027 Compile-Time Evaluation and Types](../accepted/027-compile-time-evaluation-types.md)
- [RFC-029 Module Semantics](../accepted/029-module-semantics.md)
- [RFC-036 Test Framework](../accepted/036-test-framework.md)
- [RFC-029a Module Cache and Incremental Recompilation (Draft)](../draft/029a-module-cache-incremental.md)
- `src/frontend/core/typecheck/layers/README.md` — current layer-order declaration (reverse of
  actual; D7 requires correction)
- `build.rs:19-55` — the repo's existing `panic!`-level gate exemplar
- `docs/src/dev/design/check/` — the three documents describing an architecture that never existed
  (see `06-cleanup-inventory.md` §C)
