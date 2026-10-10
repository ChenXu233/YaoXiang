# Coding Rules

> **This document is the sole authoritative source for YaoXiang code change rules.** It precisely
> describes the rules, without status diagnosis or historical justification. Historical incident
> evidence and decision rationale: see
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (October 2026 diagnosis
> records, archived with RFC-039); external convention references (Go / rustc, etc.) are also in
> that document. Execution entry point: [HOWTO.md](HOWTO.md) (pre-work self-check) → This document
> (rules) → PR template (mandatory at submission).

## Part One: Three Prohibitions

### Prohibition One: No Fabricating New Concepts

Before adding any `pub` type / enum / constant table / concept, **all four** of the following
criteria must pass for it to be compliant. Any single hit is a violation:

| Criterion                        | Rule                                                                                                                                                           |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility Overlap        | The new item's variant names overlap with some existing item by ≥ 50% and have overlapping responsibilities → Violation                                        |
| B. Insufficient Call Sites       | New `pub` item has < 2 call sites (only the definition site + the sole call site) → Violation; cannot prove it is a concept rather than a local implementation |
| C. Need for Disambiguating Alias | Need `use … as XxxBinOp`-style aliases to distinguish same-semantic items → Violation                                                                          |
| D. Relying on Synonym Tables     | Relying on hand-written string matching to bridge differences → Violation; must be changed to exhaustive matching or explicit conversion                       |

### Prohibition Two: No Accumulation of Responsibilities

A module takes on only one class of responsibility. No line count / volume / file size gates are
set—scale issues are solved by responsibility separation, not by numbers.

| Criterion                             | Rule                                                                                                                                                                                                                                            |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility Class               | The new code does not belong to the target module's **existing responsibility class** (class 2 or above) → Violation; create a new module or move existing code of that responsibility along with it. **Manual judgment, cannot be mechanized** |
| B. Bypassing the Authoritative Source | New table entry added but the sole authoritative source is not updated (hand-written copies bypassing the authoritative table) → Violation                                                                                                      |
| C. Boundary Erosion                   | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leak → Violation. **Can be mechanized, checked by CI**                                                                                                |

### Prohibition Three: Refactor, Don't Patch

| Criterion                  | Rule                                                                                                                                                                         |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous Duplication  | The same behavior appears in ≥ 2 places (aliases / conversions / stage wiring / error mapping) → Violation; must be lifted to a shared layer                                 |
| B. Excessive Fan-out       | A single change requires synchronously modifying ≥ 3 existing synonymous mappings → **Judged as an architectural change**, stop work, go through the design document process |
| C. Adding New Entry Points | Adding "the Nth entry point / table item" rather than registering it in the declarative stage table → Violation                                                              |

## Part Two: Decision Procedure (D0–D4)

Every code change passes through five gates in order. Each gate is a decidable boolean condition,
**stop on the first hit**:

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

**Trade-off Criteria (throughout D0–D4)**: Reasons for rejecting a proposal can **only** be based on
**correctness** and **readability**; "size of change surface", "whether a new mechanism is
introduced", and "implementation cost" **do not constitute grounds for rejection**. This is parallel
to, not in conflict with, Prohibition One—Prohibition One constrains "whether concepts are
duplicated", while this clause constrains "whether the rationale for the trade-off is valid":
reviewers must immediately reject proposals rejected solely on the basis of change surface or new
mechanisms.

For trade-offs that require user adjudication (architectural choice, semantic deviation,
register/don't register, breaking changes, queue-jumping), follow [AGENTS.md](../../../../AGENTS.md)
"When Requesting User Adjudication (Mandatory)" and provide all four: situation + evidence, itemized
pros and cons, clear recommendation + cost, and explicit call-out of breaking impact.

## Part Three: Red Lines (Reviewers Must Reject)

1. **Don't change code you haven't looked at**—without having opened the file being changed, without
   having grepped the relevant reference points, you may not touch it.
2. **No unimplemented stubs left in core functionality**—no new `todo!()`, `unimplemented!()`,
   `Vec::new() // Not implemented yet`-style silent discarding, or indefinite "separate issue".
   Found it, fix it, or register it in the current stage's tasks.
3. **Don't delete tests, don't loosen criteria to pass**—inability to do so is an implementation
   defect; report honestly and re-evaluate, not a reason to modify the criteria; the test count may
   only increase, not decrease.
4. **Don't use import aliases to bridge same-semantic concepts.**
5. **Don't add new `include!`, don't add new cross-layer reverse dependencies, `pub(crate)`
   cross-layer leaks may only decrease, not increase.**
6. **PR required fields must be filled in truthfully**—if you can't write down the D0 authoritative
   source module name = you didn't check, immediately rejected.

## Part Four: Division of Labor Between Machine and Human

**Mechanical issues go to tools; reviewers only look at non-mechanical issues.**

| Category                                                                                                              | Executor                                                                 |
| --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Formatting, lint, table consistency, boundary erosion (Prohibition Two C), test wiring, cross-layer dependencies      | Tool: `cargo fmt` / `clippy` / `build.rs` gate / `scripts/ci/check-*.py` |
| Responsibility class judgment (Prohibition Two A), D3's "patch or architectural change" judgment, exemption rationale | Human: PR required fields + review                                       |

## Part Five: Exemption Mechanism

- Exemptions to mechanical criteria always use `// reason: <reason>` inline comments.
- The number of exemptions itself enters the gate report and is reviewed item by item.
- Exemptions are **exceptional credentials**, not **convention**; when the same kind of exemption
  appears for the 2nd time, the correct action is to change the rule or the criterion, not to add a
  3rd exemption.

## Part Six: Code Review Checklist

Every PR's review goes through this item by item:

- [ ] **D0** Did it touch any table? Which is the sole authoritative implementation module of that
      table?
- [ ] **D1** Can the new concept be expressed with existing concepts + parameters? If not, where is
      the reason written?
- [ ] **D2** Is it semantically the same as an existing concept? Where is the conversion? Does it
      need an import alias?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Was a new entry point added rather than
      registered in the stage table? Do ≥ 3 synonymous mappings need to be changed?
- [ ] **D4** Does the new code in this change belong to the target module's existing responsibility
      class?
- [ ] **Silent Channels** For "register-consume"-type mechanisms (one side registers data, the other
      consumes, e.g., SemanticDB, proof_calls, obligation ledger), are the **direct-drive** tests on
      the producer and consumer sides delivered in the same batch? If the consumer-side test
      hand-constructs registered data while the producer side has no direct-drive evidence = the
      mechanism superficially exists but the data flow never happened, reject (already three cases:
      proof_calls, RFC-039 D54, RFC-039 D55).
- [ ] **Boundary** Has the boundary been eroded? New cross-layer dependencies / `include!` /
      `pub(crate)` cross-layer leaks?
- [ ] **Division of Labor** Have mechanical issues been caught by tools? Does the review only spend
      time on non-mechanical issues?
