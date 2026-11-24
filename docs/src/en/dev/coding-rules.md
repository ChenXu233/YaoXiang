# Coding Rules

> **This document is the sole authoritative source for YaoXiang code-change rules.** It describes
> the rules precisely and contains no diagnosis of the current state or historical rationale.
> Evidence of historical incidents and decision rationale can be found in
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (October 2026 diagnostic
> record, archived with RFC-039); references to external conventions (Go / rustc, etc.) are also in
> that document. Execution entry point: [HOWTO.md](HOWTO.md) (pre-work self-check) → this document
> (rules) → PR template (mandatory upon submission).

## Part One: Three Prohibitions

### Prohibition One: No Inventing New Concepts

Before adding any `pub` type / enum / constant table / concept, all four criteria below must pass
for the addition to be compliant. Any match constitutes a violation:

| Criterion                          | Rule                                                                                                                                                                           |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A. Overlapping responsibilities    | The variant name of the new item overlaps that of an existing item by ≥ half, and their responsibilities overlap → violation                                                   |
| B. Insufficient call sites         | The new `pub` item has < 2 call sites (only the definition + the sole call site) → violation; there is no way to prove that it is a concept rather than a local implementation |
| C. Need for a disambiguating alias | A `use … as XxxBinOp`-style alias is needed to distinguish items with the same semantics → violation                                                                           |
| D. Bridging via a synonym table    | Differences are papered over through handwritten string matching → violation; exhaustive handling or an explicit conversion is required                                        |

### Prohibition Two: No Accumulation of Responsibilities

A module carries only one category of responsibility. There is no threshold for line count / size /
file size—scale problems are resolved by separating responsibilities, not by imposing numerical
limits.

| Criterion                             | Rule                                                                                                                                                                                                                                                                                        |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility category            | The new code does not belong to the target module’s existing responsibility category (i.e., it would constitute category 2 or above) → violation; create a new module or move the existing code for that responsibility along with it. **Requires human judgment and cannot be mechanized** |
| B. Bypassing the authoritative source | A new table entry is added without updating the sole authoritative source (a handwritten copy that bypasses the authoritative table) → violation                                                                                                                                            |
| C. Boundary erosion                   | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or cross-layer `pub(crate)` leakage is added → violation. **Can be mechanized and is checked by CI**                                                                                                                          |

### Prohibition Three: A Refactor Must Not Be Replaced with a Patch

| Criterion                 | Rule                                                                                                                                                                          |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous duplication | The same behavior appears in ≥ 2 places (aliasing / conversion / stage wiring / error mapping) → violation; it must be moved to a shared layer                                |
| B. Excessive fan-out      | A single change requires synchronously modifying ≥ 3 existing synonymous mappings → **determined to be an architectural change**; stop and follow the design-document process |
| C. New entry point        | An “Nth entry / table item” is added instead of being registered in the declarative stage table → violation                                                                   |

## Part Two: Decision Procedure (D0–D4)

Every code change must pass five gates in sequence. Each gate is a decidable Boolean condition, and
**evaluation stops at the first match**:

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

**Trade-off criteria (across D0–D4):** A proposal may be rejected **only** on grounds of
**correctness** and **readability**; “the size of the change,” “whether it introduces a new
mechanism,” and “implementation cost” are **not grounds for rejection**. This operates consistently
with Prohibition One: Prohibition One constrains “whether a concept is duplicated,” while this rule
constrains “whether the grounds for a trade-off are valid.” Any proposal rejected on the grounds of
change size or a new mechanism must be sent back during review.

Trade-offs requiring a user decision (architectural choice, semantic deviation,
registration/non-registration, breaking change, ordering or queue insertion) must follow the root
`AGENTS.md` section “When to Request User Adjudication (Mandatory)” and provide all four items:
situation + evidence, item-by-item pros and cons, a clear recommendation + cost, and an explicit
callout of breaking changes.

**Routing semantic corrections (RFC-039 D59):** D0–D4 all remain in force, with one additional step:
labeling the “error type.” **Categories A/B/C** (the old behavior was **unsuccessful**: crash/stack
overflow, silent failure, silent mismatch, silent no-op, or silently incorrect answer) may be fixed
alongside the refactoring, provided that there is **red before green**, no change to the externally
observable behavior of programs that previously succeeded, registration in the same commit, and
regeneration of the baseline in the same commit followed by human review. **Categories D/E/F**
(changing the observable behavior of programs that previously succeeded, changing an external
contract table, or making an RFC-level design change) must first receive user adjudication. See
[07 §Semantic Correction Routing Criteria](architecture/07-equivalence-oracle.md) and RFC-039
decision register D59 for the expanded criteria; add a line reading “Error type (A–F)” to the
required PR block.

## Part Three: Red Lines (Rejected During Review)

1. **Do not modify code you have not reviewed**—if you have not opened the file being changed and
   have not grepped the relevant reference sites, you may not change it
2. **Do not leave core functionality as a deferred implementation**—do not introduce silent-discard
   patterns such as `todo!()`, `unimplemented!`, or `Vec::new() // Not implemented yet`, nor an
   indefinitely deferred “standalone issue.” If one is found, fix it immediately or register it as a
   task for the current stage
3. **Do not delete tests or weaken criteria to obtain a green light**—if that is impossible, the
   implementation is defective; report it truthfully and reassess. Changing the criteria is not a
   valid response. **Test admission and retirement follow Part Seven:** a test may be retired only
   if it fails all three gates (clause / medical history / proof); “delete it because it is red” is
   always rejected
4. **Do not use import aliases to reconcile concepts with the same semantics**
5. **Do not add `include!`, introduce new reverse cross-layer dependencies, or increase cross-layer
   `pub(crate)` leakage; such leakage may only decrease**
6. **The required PR block must be completed truthfully**—if the D0 authoritative source module
   cannot be named, it was not investigated, and the change must be rejected immediately; a missing
   “Error type (A–F, per D59)” entry, or one that does not match the change, must likewise be
   rejected

## Part Four: Division of Labor Between Machines and Humans

**Assign mechanical problems to tools; review should focus only on non-mechanical problems.**

| Category                                                                                                                   | Performed by                                                               |
| -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Formatting, linting, table consistency, boundary erosion (Prohibition Two C), test wiring, cross-layer dependencies        | Tools: `cargo fmt` / `clippy` / `build.rs` gates / `scripts/ci/check-*.py` |
| Responsibility-category decisions (Prohibition Two A), D3’s “patch or architectural change” decision, exemption rationales | Humans: required PR block + review                                         |

**Write corrections directly into the body; do not retain old/new comparisons**: when documentation
or comments do not match reality, change the body directly to state the facts and delete the old
wording. Do not retain the old text and then append a “revision note” or a “formerly …, now …”
comparison—readers should not have to read the incorrect version before the correct one. Evidence of
the correction process (red → green, before/after diff, adjudication number) belongs in the commit
message, not in the corrected passage. Work orders and change manifests (such as `02` §Change List)
are exempt.

## Part Five: Exemption Mechanism

- Exemptions from machine-enforced criteria must always use an inline `// reason: <reason>` comment
- The number of exemptions is included in the gate report and reviewed one by one
- An exemption is **proof of an exception**, not a norm; when the same type of exemption appears for
  the second time, the correct action is to change the rule or criterion, not to add a third
  exemption

## Part Six: Code Review Checklist

Review every PR against the following items:

- [ ] **D0** Does the change touch any table? Which module is that table’s sole authoritative
      implementation?
- [ ] **D1** Can the new concept be expressed with an existing concept + parameters? If not, where
      is the rationale documented?
- [ ] **D2** Does the new concept have the same semantics as an existing concept? Where is the
      conversion? Is an import alias needed?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Was an entry added instead of being
      registered in the stage table? Must ≥ 3 synonymous mappings be changed?
- [ ] **D4** Does the newly added code belong to the target module’s existing responsibility
      category?
- [ ] **Silent channel** For a “registration–consumption” mechanism (one side registers data and the
      other consumes it, such as SemanticDB, proof_calls, or the obligation ledger), are the
      producer-side and consumer-side **direct-drive** tests delivered in the same batch? A
      consumer-side test that manually constructs registration data while the producer side has no
      direct-drive evidence means that the mechanism exists only superficially and the data flow
      never occurred; reject it (three existing cases: proof_calls, RFC-039 D54, RFC-039 D55)
- [ ] **Boundaries** Have the boundaries been eroded? Have any cross-layer dependencies, `include!`
      calls, or cross-layer `pub(crate)` leakage been added?
- [ ] **Check admission** Which of the three gates in Part Seven does the new test / gate pass?
      Where is the evidence documented? (For a check that passes the **proof** gate, also answer:
      would it turn red with a different correct implementation?)
- [ ] **Division of labor** Are mechanical problems covered by tools? Is review spending time only
      on non-mechanical problems?

## Part Seven: Admission and Retirement of Checks and Tests

**A green light proves nothing.** A check has only negative epistemic value—it proves only that
“this time, it did not get this point wrong.” Whether a check is worth keeping depends solely on
whether it guards a **documented property**; if that cannot be answered, it is not a safety measure
but a tax on people who follow the rules.

### 7.1 The Sole Validation: Would It Turn Red with a Different Correct Implementation?

| Result               | Meaning                                                                                                                | Disposition                                      |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| It turns red         | It freezes an **implementation choice** (rendered text, message wording, exact line number, internal structural shape) | Characterize it; **do not admit it to the gate** |
| It does not turn red | It guards a **property**                                                                                               | Legitimate check                                 |

Any check that freezes the current output wholesale (full snapshots, byte-for-byte comparisons) will
necessarily turn red when replaced with a different correct implementation; by definition, it cannot
pass this gate.

Corollary: the atomic unit of a test is not “one test,” but **one falsifiable distinction**. Writing
N tests with the same code path, the same class of input, and the same assertion intent adds zero
incremental information; the fact that “this repository has thousands of tests” is not evidence in
itself.

### 7.2 Admission: Three Gates

Every new check must pass at least one of the following gates, and the evidence must be documented
in a header comment in the test file or in the commit message:

| Gate                | Required submission                                                                                                                         |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **Clause**          | Name the specific SPEC / RFC clause                                                                                                         |
| **Medical history** | Name a real failure: which change, which defect, and how it was exposed                                                                     |
| **Proof**           | ① What is the property? ② Why it **must** hold, independently of implementation choices? ③ Why is this test its cheapest observation point? |

The three gates are **not equivalent**: a clause is an external constraint and may be ambiguously
worded; medical history covers only the one incident that occurred; **proof covers the entire class
of properties**, and therefore lasts the longest—its value lies not in what it has caught before,
but in blocking an entire class of violations.

The third gate must have teeth. Everyone can say “this is an invariant,” but most things described
that way actually freeze the current implementation. All three gates share one validation (7.1): if
a check turns red with a different correct implementation, return it to characterization—it is not
proof, but a freeze disguised as proof.

### 7.3 Retirement: Reversing the Burden of Proof

Run every existing check through 7.2: keep it if it passes any one of the gates; delete it if it
passes none. There is no “observation period,” “downgrade to advisory,” or “retain after marking as
characterization”—all three merely extend the life of something whose basis cannot be explained.

- **Deletion is not irreversible**: history remains in git and can be recovered by commit when
  needed. Precisely because it can be recovered, “delete first and retrieve it if needed” is
  preferable to “keep everything unconditionally.”
- Delete in batches, with one commit per batch that can be reverted as a whole.
- Each batch’s commit message must record what was deleted, which gate was used to make the
  decision, and whether replacement coverage exists.

### 7.4 Cost Discipline

Cost is unrelated to legitimacy. Cost must be measurable, using the repository’s existing `cost:`
resource-profiling tags, and must be included in the gate budget. **Never use “CI is parallel, so it
does not matter” as a reason**—parallelism saves wall-clock time, not the time each person spends on
local pre-checks, and the latter is what is paid daily.

### 7.5 Boundary with Red Line 3

Red Line 3 addresses **motivation** (changing criteria to make CI green), while this Part addresses
**cost-benefit**; the two do not conflict.

**Adjudication order when they conflict (mandatory):** ① Is the check red now because the
**implementation is wrong**, or because the **criterion is wrong**? ② If this cannot be answered, do
not delete the check and do not change the criterion. ③ If the criterion is wrong, retire the check
through 7.3 rather than changing the assertion.
