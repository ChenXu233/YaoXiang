# Compiler Architecture Refactoring · Supplementary Design Documents

> This directory contains the supplementary design documents for
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md), at
> the same level as `../check/` and `../formatter/`.
>
> **The entry point for review is RFC-039 itself.** This directory is the implementation blueprint
> developed after the project was approved—each document maps to one layer in the RFC's main text or
> a cross-cutting concern.

## Reading Order

| Order | Document                                                                  | What to read it for                                                                                                                                |
| ----- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | [RFC-039 main text](../../rfc/accepted/039-compiler-architecture.md)      | Why to refactor, the four-layer model, criteria tiers, and the P0–P10 execution order                                                              |
| 2     | [HOWTO.md](../HOWTO.md)                                                   | **Implementer's guide**: a pre-implementation self-check list + a "patch-style fix" decision (D3). The first document to open before changing code |
| 3     | [coding-rules.md](../coding-rules.md) + [08](08-maintenance-mechanism.md) | The rules themselves (coding-rules, which remain in force long term); 08 is a 2026-10 diagnostic record                                            |
| 4     | [09-execution-wbs.md](09-execution-wbs.md)                                | **Implementation checklist**: 11 level-one / 45 level-two / 122 level-three tasks, dependencies, and a conflict registry                           |
| 5     | [01-routing.md](01-routing.md)                                            | **Where to change when adding a feature**, the target directory structure after implementation (a long-term reference)                             |
| 6     | [07-equivalence-oracle.md](07-equivalence-oracle.md)                      | How each step proves that nothing was broken                                                                                                       |
| 7     | [02 ~ 05](02-stage-contract.md)                                           | Detailed design for each layer                                                                                                                     |

## Document Inventory

| Document                                                   | Layer             | Topic                                                                                                                                                                                                                                                             | Size      |
| ---------------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| [01-routing.md](01-routing.md)                             | Cross-layer       | Feature routing tables A/B/C, dependency-direction specifications, **the target directory structure after implementation**, current-state→target mapping, naming conventions, and guidance for future extensions                                                  | 315 lines |
| [02-stage-contract.md](02-stage-contract.md)               | L1 orchestration  | Compiler stage contracts and the obligation ledger: 11 stage inconsistencies, an 8-step evidence chain documenting the gaps, 16 obligation fields, the `Stage` enum, the `Obligations` ledger, a unified `Driver`, and `ProgramKind` (including `WasmPlayground`) | 746 lines |
| [03-type-unification.md](03-type-unification.md)           | L2/L3 root causes | Type representation unification: addressing each of the 26 `ast::Type` variants, 13 forward zero-construction variants, replacing the hand-written synonym table with `NameKind`, and compile-time gates T1-T4 for type-table generation                          | 785 lines |
| [04-ssa.md](04-ssa.md)                                     | L3 IR             | SSA transformation of the intermediate representation: four categories of defects, SSA form definitions, allocator evaluation, and a 38-item change checklist                                                                                                     | 751 lines |
| [05-frontend-paradigm.md](05-frontend-paradigm.md)         | L2 frontend       | Consolidating lexical analysis into a single implementation / an unambiguous language surface + a single declarative precedence table, narrowing the change surface for operator modifications, dead-stage handling, and test suite rebuilding                    | 700 lines |
| [06-cleanup-inventory.md](06-cleanup-inventory.md)         | Global            | Dead code and paper-only design cleanup: reachability analysis methods, 1,081 lines of orphaned code across 23 files, 1,500+ lines of dead code with zero call sites, and item-by-item handling of paper-only designs                                             | 260 lines |
| [07-equivalence-oracle.md](07-equivalence-oracle.md)       | Cross-layer       | Equivalence criteria: C1-C6 tiers, three layers of criteria (IR validator / normalized snapshots / corpus differential comparison), performance baselines, and regression gates                                                                                   | 222 lines |
| [08-maintenance-mechanism.md](08-maintenance-mechanism.md) | Cross-layer       | **Repository maintenance mechanism**: three prohibitions, the D0–D4 decision procedure, a machine-checkable rule list, a code review checklist, and references to external practices (including a list of practices that must not be copied wholesale)            | 241 lines |
| [09-execution-wbs.md](09-execution-wbs.md)                 | Cross-volume      | **Multi-level implementation task table**: a three-level WBS, a mandatory serial main chain, 7 parallelizable groups, and a conflict registry (converged on RFC-039 resolutions)                                                                                  | 332 lines |

## Cross-Reference Conventions

- References within this document set: `[05-frontend-paradigm.md](05-frontend-paradigm.md)`
- References to the RFC main text: `[RFC-039](../../rfc/accepted/039-compiler-architecture.md)`
- References to other RFCs: `[RFC-013](../../rfc/accepted/013-error-code-specification.md)`

## About Numbering

These documents **are not RFCs**, have no RFC numbers, and do not participate in status tracking
through `scripts/rfc/check_tracking.py`—they are part of RFC-039 and live and die with the RFC. When
RFC-039's status becomes `rejected` or `deprecated`, this directory should be archived alongside it.

`01-routing.md` is the only exception: it records **facts about the current code** (which reverse
dependencies exist and which gates are missing), rather than the proposal content of RFC-039. Even
if RFC-039 is rejected, this routing table remains valid—it answers "what needs to be changed to add
a feature now."
