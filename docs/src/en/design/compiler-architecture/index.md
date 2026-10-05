# Compiler Architecture Redesign · Companion Design Documents

> This directory is the collection of companion design documents for
> [RFC-039 Compiler Architecture Redesign](../rfc/draft/039-compiler-architecture.md), sitting
> alongside `../check/` and `../formatter/`.
>
> **The review entry point is RFC-039 itself.** This directory is the construction blueprint after
> the project is established — each document corresponds to one layer or one cross-cutting concern
> in the main RFC body.

## Reading Order

| Order | Document                                                                      | What problem it solves                                                                                                                 |
| ----- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | [RFC-039 Main Body](../rfc/draft/039-compiler-architecture.md)                | Why the redesign, the four-layer model, oracle tiering, P0–P10 execution order                                                         |
| 2     | [HOWTO.md](HOWTO.md)                                                          | **Implementer's handbook**: pre-work self-checklist + "patch-style fix" judgment (D3). The first document to open before changing code |
| 3     | [coding-rules.md](../dev/coding-rules.md) + [08](08-maintenance-mechanism.md) | The rules themselves (coding-rules, long-term validity); 08 is the October 2026 diagnostic record                                      |
| 4     | [09-execution-wbs.md](09-execution-wbs.md)                                    | **Construction checklist**: 11 Level-1 / 45 Level-2 / 122 Level-3 tasks, dependencies, conflict registry                               |
| 5     | [01-routing.md](01-routing.md)                                                | **Where to change when adding a feature**, target directory structure after construction (long-term reference)                         |
| 6     | [07-equivalence-oracle.md](07-equivalence-oracle.md)                          | What proves nothing was broken at each step                                                                                            |
| 7     | [02 ~ 05](02-stage-contract.md)                                               | Concrete design for each layer                                                                                                         |

## Document Inventory

| Document                                                   | Layer            | Topic                                                                                                                                                                                                                           | Scale     |
| ---------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| [01-routing.md](01-routing.md)                             | Cross-layer      | Feature routing table A/B/C, dependency direction specification, **target directory structure after construction**, current→target mapping, naming conventions, future extension guide                                          | 315 lines |
| [02-stage-contract.md](02-stage-contract.md)               | L1 Orchestration | Compilation stage contracts and obligation ledger: 11 stage inconsistencies, 8-step leak evidence chain, 16 obligation fields, `Stage` enum, `Obligations` ledger, unified `Driver`, `ProgramKind` (including `WasmPlayground`) | 746 lines |
| [03-type-unification.md](03-type-unification.md)           | L2/L3 Root Cause | Type representation unification: 26 `ast::Type` variants handled one by one, 13 forward zero-construction variants, `NameKind` replacing handwritten synonym table, type table generation phase gates T1–T4                     | 785 lines |
| [04-ssa.md](04-ssa.md)                                     | L3 IR            | Intermediate representation SSA-ification: four defect categories, SSA shape definition, allocator evaluation, 38-item change list                                                                                              | 751 lines |
| [05-frontend-paradigm.md](05-frontend-paradigm.md)         | L2 Frontend      | Lexer convergence to sole implementation / syntax LALRPOP grammar-driven, operator change blast-radius convergence, dead-branch handling, test rebuild                                                                          | 792 lines |
| [06-cleanup-inventory.md](06-cleanup-inventory.md)         | Global           | Dead code and empty-design cleanup: reachability analysis method, 23 files / 1081 lines of orphans, 1500+ lines of zero-call dead code, empty designs handled item by item                                                      | 260 lines |
| [07-equivalence-oracle.md](07-equivalence-oracle.md)       | Cross-layer      | Equivalence oracle: C1–C6 tiering, three-layer oracle (IR validator / normalized snapshot / corpus diff), performance baseline, regression gates                                                                                | 222 lines |
| [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | Cross-layer      | **Repository maintenance mechanism**: three prohibitions, D0–D4 decision procedure, machine-checkable rule list, code review checklist, external convention references (including a "do not copy" list)                         | 241 lines |
| [09-execution-wbs.md](09-execution-wbs.md)                 | Cross-fascicle   | **Multi-level construction task table**: three-level WBS, mandatory serial main chain, 7 parallelizable groups, conflict registry (converged to RFC-039 resolutions)                                                            | 332 lines |

## Cross-Reference Conventions

- Reference within this document collection: `[05-frontend-paradigm.md](05-frontend-paradigm.md)`
- Reference RFC main body: `[RFC-039](../rfc/draft/039-compiler-architecture.md)`
- Reference other RFCs: `[RFC-013](../rfc/accepted/013-error-code-specification.md)`

## On Numbering

These documents **are not RFCs**, have no RFC numbers, and do not participate in
`scripts/rfc/check_tracking.py` status tracking — they are components of RFC-039, living and dying
with the RFC. When RFC-039 status changes to `rejected` or `deprecated`, this directory should be
archived together.

`01-routing.md` is the sole exception: it records **facts about the current code** (which reverse
dependencies exist, which gates are missing), not RFC-039 proposal content. Even if RFC-039 is
rejected, this routing table still stands — it answers the question "where do I need to change
things to add a feature right now".
