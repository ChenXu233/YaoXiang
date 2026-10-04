# Compiler Architecture Refactoring · Supporting Design Documents

> This directory is the supporting design document collection for
> [RFC-039 Compiler Architecture Refactoring](../rfc/draft/039-compiler-architecture.md), at the
> same level as `../check/` and `../formatter/`.
>
> **The review entry point is RFC-039 itself.** This directory is the construction blueprint after
> the project has been approved — each document corresponds to one layer or one cross-cutting
> concern in the RFC body.

## Reading Order

| Order | Document                                                   | What problem it solves                                                                                                   |
| ----- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| 1     | [RFC-039 Body](../rfc/draft/039-compiler-architecture.md)  | Why refactor, four-layer model, criteria grading, P0–P10 execution order                                                 |
| 2     | [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | **Rules that must be read before changing code**: three prohibitions + D0–D4 decision procedure                          |
| 3     | [09-execution-wbs.md](09-execution-wbs.md)                 | **Construction checklist**: 11 level-1 / 45 level-2 / 122 level-3 tasks, dependencies, conflict registry                 |
| 4     | [01-routing.md](01-routing.md)                             | **Where to change when adding a feature**, target directory structure after construction completes (long-term reference) |
| 5     | [07-equivalence-oracle.md](07-equivalence-oracle.md)       | What proves each step hasn't broken anything                                                                             |
| 6     | [02 ~ 05](02-stage-contract.md)                            | Specific design for each layer                                                                                           |

## Document List

| Document                                                   | Layer            | Topic                                                                                                                                                                                                                                    | Scale     |
| ---------------------------------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| [01-routing.md](01-routing.md)                             | Cross-layer      | Feature routing table A/B/C, dependency direction specification, **target directory structure after construction completes**, current→target mapping, naming conventions, future extension guidelines                                    | 315 lines |
| [02-stage-contract.md](02-stage-contract.md)               | L1 Orchestration | Compilation stage contracts and obligation ledger: 11 stage inconsistencies, 8-step vulnerability evidence chain, 16 obligation fields, `Stage` enum, `Obligations` ledger, unified `Driver`, `ProgramKind` (including `WasmPlayground`) | 746 lines |
| [03-type-unification.md](03-type-unification.md)           | L2/L3 root cause | Unification of type representation: 26 `ast::Type` variants handled one by one, 13 forward zero-construction variants, `NameKind` replaces hand-written synonym table, type table generation-time gates T1-T4                            | 785 lines |
| [04-ssa.md](04-ssa.md)                                     | L3 IR            | SSA-ification of intermediate representation: four defect categories, SSA form definition, allocator evaluation, 38-item change list                                                                                                     | 751 lines |
| [05-frontend-paradigm.md](05-frontend-paradigm.md)         | L2 Frontend      | Lexical convergence to a single implementation / syntax driven by LALRPOP grammar, operator change impact surface convergence, dead step handling, test reconstruction                                                                   | 792 lines |
| [06-cleanup-inventory.md](06-cleanup-inventory.md)         | Global           | Dead code and empty-design cleanup: reachability analysis method, 23 files 1081 lines of orphans, 1500+ lines of zero-call dead code, empty design handled item by item                                                                  | 260 lines |
| [07-equivalence-oracle.md](07-equivalence-oracle.md)       | Cross-layer      | Equivalence criteria: C1-C6 grading, three-tier criteria (IR validator / normalized snapshot / corpus diff), performance baseline, regression gate                                                                                       | 222 lines |
| [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | Cross-layer      | **Repository maintenance mechanism**: three prohibitions, D0–D4 decision procedure, machine-checkable rule list, code review checklist, external convention reference (including non-portable list)                                      | 241 lines |
| [09-execution-wbs.md](09-execution-wbs.md)                 | Cross-volume     | **Multi-level construction task table**: three-level WBS, mandatory serial main chain, 7 parallelizable groups, conflict registry (converged to RFC-039 resolution)                                                                      | 332 lines |

## Cross-Reference Conventions

- Reference within this document collection: `[05-frontend-paradigm.md](05-frontend-paradigm.md)`
- Reference to RFC body: `[RFC-039](../rfc/draft/039-compiler-architecture.md)`
- Reference to other RFCs: `[RFC-013](../rfc/accepted/013-error-code-specification.md)`

## About Numbering

These documents **are not RFCs**, have no RFC number, and do not participate in the status tracking
of `scripts/rfc/check_tracking.py` — they are part of RFC-039, living and dying with the RFC. When
the status of RFC-039 changes to `rejected` or `deprecated`, this directory should be archived
together.

`01-routing.md` is the only exception: it records **facts about the current code** (which reverse
dependencies exist, which gates are missing), not the proposal content of RFC-039. Even if RFC-039
is rejected, this routing table still holds — it answers "where to change now to add a feature".
