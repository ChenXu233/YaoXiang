# Coding Rules

> **This document is the sole authoritative body for YaoXiang code change rules.** It precisely
> describes the rules, with no status diagnosis or historical justification. Historical incident
> evidence and decision rationale are in
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (2026-10 diagnostic
> record, archived with RFC-039); references to external conventions (Go / rustc, etc.) are in the
> same document. Execution entry: [HOWTO.md](HOWTO.md) (pre-work self-check) → This document (rules)
> → PR template (mandatory at submission).

## Part One: Three Prohibitions

### Prohibition One: No Invention

Before adding any `pub` type / enum / constant table / concept, all four of the following criteria
must pass to be compliant. Any hit is a violation:

| Criterion                       | Rule                                                                                                                                                                 |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Overlapping responsibilities | New item shares variant names with an existing item by ≥ 50% AND has overlapping responsibilities → violation                                                        |
| B. Insufficient call sites      | The new `pub` item has < 2 call sites (only the definition site + the single call site) → violation; cannot prove it is a concept rather than a local implementation |
| C. Disambiguating alias needed  | A `use … as XxxBinOp`-style alias is required to distinguish items of the same semantics → violation                                                                 |
| D. Bridged by synonym table     | Differences are reconciled by hand-written string matching → violation, must be exhaustive enumeration or explicit conversion                                        |

### Prohibition Two: No Responsibility Accumulation

A module takes on only one class of responsibility. No line / volume / file-size gate — scale
problems are solved by responsibility separation, not by numbers.

| Criterion                         | Rule                                                                                                                                                                                                                                         |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility class           | New code does not belong to the target module's **existing responsibility class** (class 2 and above) → violation; create a new module, or move the existing code of that responsibility together. **Human judgment, not machine-checkable** |
| B. Bypassing authoritative source | New table entry without updating the sole authoritative source (hand-writing copies, bypassing the authoritative table) → violation                                                                                                          |
| C. Boundary erosion               | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, `pub(crate)` cross-layer leakage → violation. **Machine-checkable, by CI**                                                                                                     |

### Prohibition Three: No Patching in Lieu of Refactoring

| Criterion                 | Rule                                                                                                                                                             |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous duplication | The same behavior appears in ≥ 2 places (alias / conversion / phase wiring / error mapping) → violation; must be lifted to a shared layer                        |
| B. Excessive fan-out      | One change must synchronously update ≥ 3 existing synonymous mappings → **judged as an architectural change**, stop work, go through the design document process |
| C. New entry point        | Adding "the Nth entry / table item" instead of registering into the declarative phase table → violation                                                          |

## Part Two: Decision Procedure (D0–D4)

Each code change goes through five gates in order. Each gate is a decidable boolean condition,
**stop on hit**:

```
D0  门禁    本次改动是否触碰任何一张已存在的表（错误码/opcode/类型/入口阶段）？
             ├─ 是 → 能指出该表的唯一权威实现模块吗？
             │        ├─ 能 → 继续
             │        └─ 不能 → ⛔ 停止。先建立权威源
             └─ 否 → 继续
                    ↓
D1  防生造   去掉新概念后，能否用「既有概念 + 参数」表达同样的语义？
             ├─ 能 → ⛔ 禁止新增。理由记入 PR
             └─ 不能 → 继续
                    ↓
D2  防并行   新概念是否与某既有概念同语义（变体名重合 ≥ 半数）？
             ├─ 是 → 有 From/TryFrom 转换且机检吗？
             │        ├─ 有 → 继续
             │        └─ 无 → ⛔ 合并为一份（禁止用 import 别名区分）
             └─ 否 → 继续
                    ↓
D3  补丁?    依次判定，命中即止：
             1) 同一行为需在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层
             2) 需要新增"第 6 个入口"？         → 是 → ⛔ 禁止手工接线，改为登记进阶段表
             3) 一次修改要改 ≥3 处同义映射？   → 是 → ⛔ 判定为架构变更，停工走设计文档
             4) 以上皆否                       → 允许局部补丁，但必须附带回归测试
                    ↓
D4  职责?    本次新增代码属于目标模块已有的职责类别吗？
             ├─ 否（第 2 类及以上）→ ⛔ 新建模块，或把该职责的现有代码一并搬过去
             └─ 是 → 合规
```

**Trade-off criteria (throughout D0–D4)**: The reason for rejecting a plan **may only** be based on
**correctness** and **readability**; "size of change footprint", "whether a new mechanism is
introduced", "implementation cost" **are not grounds for rejection**. This runs parallel to
Prohibition One — Prohibition One constrains "whether the concept is duplicated", this one
constrains "whether the trade-off reason holds": those who reject a plan on the grounds of change
footprint or a new mechanism, have their review bounced back directly.

Choices that require user adjudication (architectural choices, semantic deviations, register /
do-not-register, breaking changes, sorting queue-jumping), follow the repository-root `AGENTS.md`
"When requesting user adjudication (mandatory)" with all four items: situation + evidence,
item-by-item pros and cons, explicit recommendation + cost, breaking-change callout.

**Triage for semantic corrections (RFC-039 D59)**: D0–D4 all retained; one extra step of labeling
the "error type" — **classes A/B/C** (old behavior is **non-success**: crash / stack overflow,
silent failure, silent mismatch, silent inaction, silently giving a wrong answer) **may be fixed in
passing during refactoring**, provided that **red-before-green + no change in the externally
observable behavior of programs that "previously succeeded" + same-commit registration + baseline
regeneration in the same commit plus manual review**; **classes D/E/F** (changing the observable
behavior of programs that previously succeeded, external contract tables, RFC-level design changes)
**must go through user adjudication first**. Criteria expansion is in
[07 §Triage Criteria for Semantic Corrections](architecture/07-equivalence-oracle.md) and RFC-039
resolution entry D59; the PR required block adds one line "Error type (A–F)".

## Part Three: Red Lines (review must bounce back)

1. **Don't change code you haven't read** — without opening the changed file, without grepping
   related reference points, you must not touch it
2. **No un-implemented core functionality left behind** — no new `todo!()`, `unimplemented!`,
   `Vec::new() // Not implemented yet`-style silent drops, or indefinite "separate issue". Found and
   fix on the spot, or register into the current phase's tasks
3. **Don't delete tests, don't loosen criteria for green** — inability is an implementation defect,
   report truthfully and re-evaluate; modifying the criteria is not a reason; test count may only
   increase, never decrease
4. **Don't use import aliases to bridge concepts of the same semantics**
5. **No new `include!`, no new cross-layer reverse dependencies; `pub(crate)` cross-layer leakage
   may only decrease, never increase**
6. **PR required blocks must be filled in truthfully** — inability to write the D0
   authoritative-source module name = you didn't look it up, bounce back directly; "Error type (A–F,
   per D59)" missing or inconsistent with the actual change, also bounce back

## Part Four: Division of Labor Between Machine and Human

**Mechanical problems go to tools; review focuses only on non-mechanical problems.**

| Category                                                                                                          | Executor                                                                  |
| ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Formatting, lint, table consistency, boundary erosion (Prohibition Two C), test wiring, cross-layer dependencies  | Tools: `cargo fmt` / `clippy` / `build.rs` gate / `scripts/ci/check-*.py` |
| Responsibility-class judgment (Prohibition Two A), D3 "patch or architectural change" judgment, exemption reasons | Human: PR required blocks + review                                        |

## Part Five: Exemption Mechanism

- Machine-criterion exemptions uniformly use `// reason: <reason>` inline comments
- The number of exemptions itself enters the gate report, reviewed item by item
- Exemptions are **exception credentials**, not **convention**; when the same kind of exemption
  appears a 2nd time, the correct action is to change the rule or the criterion, not to add a 3rd
  exemption

## Part Six: Code Review Checklist

Review each PR against this checklist item by item:

- [ ] **D0** Does it touch any table? What is the unique authoritative implementation module of that
      table?
- [ ] **D1** Can the new concept be expressed with an existing concept + parameters? If not, where
      is the reason written?
- [ ] **D2** Does it share semantics with an existing concept? Where is the conversion? Does it need
      an import alias?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Was an entry added instead of registering
      into the phase table? Does the change need to update ≥ 3 synonymous mappings?
- [ ] **D4** Does the newly added code belong to the target module's existing responsibility class?
- [ ] **Silent channel** For "register–consume" mechanisms (one side registers data, the other side
      consumes, e.g., SemanticDB, proof_calls, obligation ledger), are direct-drive tests of the
      producer end and the consumer end delivered in the same batch? If consumer-end tests
      hand-construct registered data while the producer end has no direct-drive evidence = the
      mechanism appears on the surface, but the data flow never actually happens; bounce back
      (already three cases: proof_calls, RFC-039 D54, RFC-039 D55)
- [ ] **Boundary** Is it being eroded? New cross-layer dependency / `include!` / `pub(crate)`
      cross-layer leakage?
- [ ] **Division of labor** Are mechanical problems already covered by tools? Does the review spend
      time only on non-mechanical issues?
