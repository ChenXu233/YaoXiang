# Compiler Architecture Refactor · Supplementary Design Documents

> This directory is a collection of supplementary design documents for
> [RFC-039: Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md), at the
> same level as `../check/` and `../formatter/`.
>
> **The entry point for review is RFC-039 itself.** This directory contains the construction
> blueprints after the project is approved—each document corresponds to one layer or one
> cross-cutting concern in the RFC body.

## Reading Order

| Order | Document                                                                  | What problem does it solve                                                                                                         |
| ----- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| 1     | [RFC-039 body](../../rfc/accepted/039-compiler-architecture.md)           | Why refactor, four-layer model, criteria tiers, P0–P10 execution order                                                             |
| 2     | [HOWTO.md](../HOWTO.md)                                                   | **Implementer's handbook**: pre-work self-check + "patch-style fix" judgment (D3). The first document to open before changing code |
| 3     | [coding-rules.md](../coding-rules.md) + [08](08-maintenance-mechanism.md) | The rules themselves (coding-rules, long-term validity); 08 is the October 2026 diagnostic record                                  |
| 4     | [09-execution-wbs.md](09-execution-wbs.md)                                | **Construction checklist**: 11 first-level / 45 second-level / 122 third-level tasks, dependencies, conflict log                   |
| 5     | [01-routing.md](01-routing.md)                                            | **Where to change when adding a feature**, target directory structure after construction (long-term reference)                     |
| 6     | [07-equivalence-oracle.md](07-equivalence-oracle.md)                      | What proves nothing is broken at each step                                                                                         |
| 7     | [02 ~ 05](02-stage-contract.md)                                           | Specific design for each layer                                                                                                     |

## Document Inventory

| Document                                                   | Layer            | Topic                                                                                                                                                                                                                                    | Size      |
| ---------------------------------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| [01-routing.md](01-routing.md)                             | Cross-layer      | Feature routing table A/B/C, dependency direction specification, **target directory structure after construction**, current→target mapping, naming conventions, future extension guide                                                   | 315 lines |
| [02-stage-contract.md](02-stage-contract.md)               | L1 orchestration | Compilation stage contracts and obligation ledger: 11 stage inconsistencies, 8-step vulnerability evidence chain, 16 obligation fields, `Stage` enum, `Obligations` ledger, unified `Driver`, `ProgramKind` (including `WasmPlayground`) | 746 lines |
| [03-type-unification.md](03-type-unification.md)           | L2/L3 root cause | Type representation unification: 26 `ast::Type` variants handled one by one, 13 forward zero-construction variants, `NameKind` replaces hand-written synonym tables, type table generation gating T1-T4                                  | 785 lines |
| [04-ssa.md](04-ssa.md)                                     | L3 IR            | SSA-ification of intermediate representation: four types of defects, SSA shape definition, allocator evaluation, 38-item change list                                                                                                     | 751 lines |
| [05-frontend-paradigm.md](05-frontend-paradigm.md)         | L2 frontend      | Lexical convergence to a single implementation / syntax LALRPOP grammar-driven, operator change surface convergence, dead step handling, test rebuild                                                                                    | 792 lines |
| [06-cleanup-inventory.md](06-cleanup-inventory.md)         | Global           | Dead code and empty design cleanup: reachability analysis method, 23 files 1081 lines of orphans, 1500+ lines of zero-call dead code, empty design disposition item by item                                                              | 260 lines |
| [07-equivalence-oracle.md](07-equivalence-oracle.md)       | Cross-layer      | Equivalence criteria: C1-C6 tiers, three-layer criteria (IR validator / canonicalized snapshot / corpus diff), performance baseline, regression gating                                                                                   | 222 lines |
| [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | Cross-layer      | **Repository maintenance mechanism**: three prohibitions, D0–D4 decision procedure, machine-checkable rule list, code review checklist, external convention references (including non-importable list)                                   | 241 lines |
| [09-execution-wbs.md](09-execution-wbs.md)                 | Cross-volume     | **Multi-level construction task table**: three-level WBS, mandatory serial main chain, 7 parallelizable groups, conflict log (converged into RFC-039 resolutions)                                                                        | 332 lines |

## Cross-Reference Conventions

- References within this document collection: `[05-frontend-paradigm.md](05-frontend-paradigm.md)`
- References to RFC body: `[RFC-039](../../rfc/accepted/039-compiler-architecture.md)`
- References to other RFCs: `[RFC-013](../../rfc/accepted/013-error-code-specification.md)`

## On Numbering

These documents are **not RFCs**, have no RFC numbers, and do not participate in the status tracking
of `scripts/rfc/check_tracking.py`—they are components of RFC-039 and live and die with the RFC.
When RFC-039's status changes to `rejected` or `deprecated`, this directory should be archived along
with it.

`01-routing.md` is the only exception: it records **facts about the current code** (which reverse
dependencies exist, which gates are missing), not the proposal content of RFC-039. Even if RFC-039
is rejected, this routing table still holds—it answers "where to change in the current code to add a
feature".
