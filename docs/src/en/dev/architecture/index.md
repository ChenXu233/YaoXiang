# Compiler Architecture Refactoring · Companion Design Documents

> This directory is the companion design document collection for
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md), at the
> same level as `../check/` and `../formatter/`.
>
> **The review entry point is RFC-039 itself.** This directory is the construction blueprint after
> the project is approved — each document corresponds to one layer or one cross-cutting concern in
> the RFC body.

## Reading Order

| Order | Document                                                                  | What problem does reading it solve                                                                                                        |
| ----- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | [RFC-039 body](../../rfc/draft/039-compiler-architecture.md)              | Why refactor, the four-layer model, criterion grading, P0–P10 execution order                                                             |
| 2     | [HOWTO.md](../HOWTO.md)                                                   | **Implementer's manual**: pre-work self-checklist + "patch-style fix" determination (D3). The first document to open before touching code |
| 3     | [coding-rules.md](../coding-rules.md) + [08](08-maintenance-mechanism.md) | The rules themselves (coding-rules, long-term validity); 08 is the October 2026 diagnostic record                                         |
| 4     | [09-execution-wbs.md](09-execution-wbs.md)                                | **Construction checklist**: 11 Level-1 / 45 Level-2 / 122 Level-3 tasks, dependencies, conflict log                                       |
| 5     | [01-routing.md](01-routing.md)                                            | **Where to change to add a feature**, the target directory structure after construction (long-term reference)                             |
| 6     | [07-equivalence-oracle.md](07-equivalence-oracle.md)                      | What proves we haven't broken things at each step                                                                                         |
| 7     | [02 ~ 05](02-stage-contract.md)                                           | Specific designs for each layer                                                                                                           |

## Document Checklist

| Document                                                   | Layer            | Topic                                                                                                                                                                                                                                 | Size      |
| ---------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| [01-routing.md](01-routing.md)                             | Cross-layer      | Feature routing table A/B/C, dependency direction specification, **target directory structure after construction**, current → target mapping, naming conventions, future extension guide                                              | 315 lines |
| [02-stage-contract.md](02-stage-contract.md)               | L1 Orchestration | Compile-stage contracts and obligations ledger: 11 stage inconsistencies, 8-step vulnerability evidence chain, 16 obligation fields, `Stage` enum, `Obligations` ledger, unified `Driver`, `ProgramKind` (including `WasmPlayground`) | 746 lines |
| [03-type-unification.md](03-type-unification.md)           | L2/L3 Root Cause | Unification of type representation: 26 `ast::Type` variants handled one by one, 13 forward zero-construct variants, `NameKind` replaces hand-written synonym tables, type-table generation-time gates T1–T4                           | 785 lines |
| [04-ssa.md](04-ssa.md)                                     | L3 IR            | Intermediate representation SSA-ification: four classes of defects, SSA form definition, allocator evaluation, 38-item change checklist                                                                                               | 751 lines |
| [05-frontend-paradigm.md](05-frontend-paradigm.md)         | L2 Frontend      | Lexer convergence to the sole implementation / LALRPOP grammar-driven parser, operator-change touchpoint convergence, dead-stair handling, test rebuild                                                                               | 792 lines |
| [06-cleanup-inventory.md](06-cleanup-inventory.md)         | Global           | Dead code and empty-design cleanup: reachability analysis method, 23 files with 1081 lines of orphans, 1500+ lines of zero-call dead code, empty-design item-by-item disposition                                                      | 260 lines |
| [07-equivalence-oracle.md](07-equivalence-oracle.md)       | Cross-layer      | Equivalence criteria: C1–C6 grading, three-tier criteria (IR validator / canonicalized snapshots / corpus diff), performance baseline, regression gate                                                                                | 222 lines |
| [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | Cross-layer      | **Repository maintenance mechanism**: three prohibitions, D0–D4 decision procedure, machine-checkable rule list, code review checklist, external convention references (including a "do not copy" list)                               | 241 lines |
| [09-execution-wbs.md](09-execution-wbs.md)                 | Cross-booklet    | **Multi-level construction task table**: three-level WBS, mandatory serial main chain, 7 parallelizable groups, conflict log (converged into RFC-039 resolutions)                                                                     | 332 lines |

## Cross-Reference Conventions

- Reference within this document set: `[05-frontend-paradigm.md](05-frontend-paradigm.md)`
- Reference to RFC body: `[RFC-039](../../rfc/draft/039-compiler-architecture.md)`
- Reference to other RFCs: `[RFC-013](../../rfc/accepted/013-error-code-specification.md)`

## About Numbering

These documents are **not RFCs** — they have no RFC number, and they do not participate in the
status tracking of `scripts/rfc/check_tracking.py`. They are part of RFC-039, and live and die with
it. When RFC-039's status becomes `rejected` or `deprecated`, this directory should be archived
together.

`01-routing.md` is the sole exception: it records **facts about the current code** (which reverse
dependencies exist, which gates are missing), not the proposal content of RFC-039. Even if RFC-039
is rejected, this routing table still holds — it answers the question "to add a feature right now,
which places need to be changed".
