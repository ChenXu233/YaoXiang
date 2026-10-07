# Coding Rules

> **This document is the sole authoritative source for YaoXiang code change rules.** It precisely
> describes the rules, without diagnosis of current state or historical argumentation. Historical
> incident evidence and decision rationale are in
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (2026-10 diagnosis
> records, archived with RFC-039); external convention references (Go / rustc, etc.) are in the same
> document. Execution entry point: [HOWTO.md](HOWTO.md) (pre-work self-check) → this document
> (rules) → PR template (mandatory at submission time).

## Part One: Three Prohibitions

### Prohibition 1: No Inventing

Before adding any `pub` type / enum / constant table / concept, the following four criteria **must
all pass** to be compliant. Any one hit is a violation:

| Criterion                     | Rule                                                                                                                                                             |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Overlapping responsibility | New item shares ≥ half of its variant names with an existing item AND has overlapping responsibility → violation                                                 |
| B. Insufficient call sites    | New `pub` item has < 2 call sites (only the definition site + the single call site) → violation; cannot prove it is a concept rather than a local implementation |
| C. Needs disambiguation alias | Needs `use … as XxxBinOp`-style aliases to distinguish items with the same semantics → violation                                                                 |
| D. Bridged by synonym table   | Bridges differences through hand-written string matching → violation; must be changed to exhaustive enumeration or explicit conversion                           |

### Prohibition 2: No Responsibility Accumulation

A module only takes on one class of responsibility. No line count / volume / file size gate—scale
issues are solved by responsibility separation, not by numbers.

| Criterion                             | Rule                                                                                                                                                                                                                                                   |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A. Responsibility class               | New code does not belong to the **existing responsibility class** of the target module (class 2 or above) → violation, create a new module or move the existing code for that responsibility along with it. **Manual judgment, not machine-checkable** |
| B. Bypassing the authoritative source | New table entry added but the unique authoritative source is not updated (hand-written copy bypassing the authoritative table) → violation                                                                                                             |
| C. Boundary erosion                   | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, `pub(crate)` cross-layer leakage → violation. **Machine-checkable, enforced by CI**                                                                                                      |

### Prohibition 3: No Patching What Should Be Refactored

| Criterion                 | Rule                                                                                                                                                                             |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous duplication | The same behavior appears in ≥ 2 places (alias / conversion / stage wiring / error mapping) → violation, must be lifted to the shared layer                                      |
| B. Excessive fan-out      | One modification requires synchronous changes to ≥ 3 existing synonymous mappings → **determined as an architectural change**, stop work, go through the design document process |
| C. New entry point        | Adding "the Nth entry / table entry" instead of registering in the declarative stage table → violation                                                                           |

## Part Two: Decision Procedure (D0–D4)

Each code change passes through five gates in order. Each gate is a decidable boolean condition,
**stops on hit**:

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

## Part Three: Red Lines (Review Must Reject)

1. **Don't change code you haven't looked at** — without opening the modified file, without grepping
   for related reference points, you are not allowed to touch it
2. **No "to be implemented" remnants in core functionality** — no new `todo!()`, `unimplemented!()`,
   `Vec::new() // Not implemented yet`-style silent discarding, or indefinite "separate issue". Find
   and fix when discovered, or register in the current stage's tasks
3. **Don't delete tests, don't loosen criteria for green lights** — being unable to do it is an
   implementation defect; report honestly and reassess—this is not a reason to modify the criteria;
   the number of tests can only increase, not decrease
4. **Don't use import aliases to bridge concepts with the same semantics**
5. **Don't add `include!`, don't add cross-layer reverse dependencies, `pub(crate)` cross-layer
   leakage can only decrease, not increase**
6. **PR required fields must be filled in truthfully** — if the D0 authoritative source module name
   cannot be written, it means you didn't check; reject directly

## Part Four: Division of Labor Between Machine and Human

**Mechanical problems go to tools; review only looks at non-mechanical problems.**

| Category                                                                                                               | Executor                                                                  |
| ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Formatting, lint, table consistency, boundary erosion (Prohibition 2 C), test wiring, cross-layer dependencies         | Tools: `cargo fmt` / `clippy` / `build.rs` gate / `scripts/ci/check-*.py` |
| Responsibility class determination (Prohibition 2 A), D3's "patch or architectural change" judgment, exemption reasons | Human: PR required fields + review                                        |

## Part Five: Exemption Mechanism

- Exemptions for machine-checkable criteria uniformly use `// reason: <reason>` inline comments
- The number of exemptions itself is included in the gate report and is reviewed item by item
- Exemptions are **exception vouchers**, not **convention**; when the same kind of exemption appears
  for the 2nd time, the correct action is to change the rule or the criterion, not to add a 3rd
  exemption

## Part Six: Code Review Checklist

Each PR's review goes through these items one by one:

- [ ] **D0** Does it touch any table? What is the unique authoritative implementation module of that
      table?
- [ ] **D1** Can the new concept be expressed with an existing concept + parameters? If not, where
      is the reason written?
- [ ] **D2** Does it have the same semantics as an existing concept? Where is the conversion? Does
      it need an import alias?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Did it add an entry point instead of
      registering in the stage table? Does it need to change ≥ 3 synonymous mappings?
- [ ] **D4** Does the new code added this time belong to the existing responsibility class of the
      target module?
- [ ] **Silent channel** Are the producer-side and consumer-side **direct-driven** tests of
      "register-consume"-type mechanisms (one party registers data, the other consumes, such as
      SemanticDB, proof_calls, duty ledgers) delivered in the same batch? Consumer-side tests
      manually construct registered data while the producer-side has no direct-driven evidence = the
      mechanism appears to exist but the data flow never happened; reject (three cases so far:
      proof_calls, RFC-039 D54, RFC-039 D55)
- [ ] **Boundary** Has it been eroded? New cross-layer dependencies / `include!` / `pub(crate)`
      cross-layer leakage?
- [ ] **Division of labor** Have mechanical problems been caught by tools? Is the review only spent
      on non-mechanical problems?
