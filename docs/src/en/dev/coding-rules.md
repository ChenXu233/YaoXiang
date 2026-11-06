# Coding Rules

> **This document is the sole authoritative source for YaoXiang code-change rules.** It describes
> the rules precisely, without diagnosing the current state or providing historical rationale.
> Historical incident evidence and decision rationale are documented in
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (October 2026 diagnostic
> record, archived with RFC-039); references to external conventions (Go / rustc, etc.) are also
> found there. Execution path: [HOWTO.md](HOWTO.md) (pre-work self-check) → this document (rules) →
> PR template (mandatory at submission).

## Part One: Three Prohibitions

### Prohibition One: No Inventing Concepts

Before adding any `pub` type / enum / constant table / concept, **all four** criteria below must
pass for the addition to be compliant. Any violation is prohibited:

| Criterion                                   | Rule                                                                                                                                                                             |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Overlapping responsibility               | The new item's variant names overlap by at least half with those of an existing item, and their responsibilities overlap → violation                                             |
| B. Insufficient call sites                  | The new `pub` item has fewer than two call sites (only the definition and its sole call site) → violation; it cannot be shown to be a concept rather than a local implementation |
| C. Disambiguating alias required            | An alias such as `use … as XxxBinOp` is needed to distinguish semantically equivalent items → violation                                                                          |
| D. Differences bridged with a synonym table | Differences are bridged through handwritten string matching → violation; exhaustive matching or an explicit conversion is required                                               |

### Prohibition Two: No Accumulation of Responsibilities

A module has only one kind of responsibility. There is no line-count, size, or file-size gate:
scaling problems are solved by separating responsibilities, not by imposing numerical limits.

| Criterion                             | Rule                                                                                                                                                                                                                                                                                                   |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A. Responsibility category            | The new code does not belong to the target module's **existing responsibility category** (that is, it would introduce a second or subsequent category) → violation; create a new module or move the existing code for that responsibility as well. **Requires human judgment and cannot be automated** |
| B. Bypassing the authoritative source | A table entry is added without updating the sole authoritative source (a handwritten copy bypasses the authoritative table) → violation                                                                                                                                                                |
| C. Boundary erosion                   | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or cross-layer `pub(crate)` exposure is introduced → violation. **Can be automated and is checked by CI**                                                                                                                                |

### Prohibition Three: No Patching Instead of Refactoring

| Criterion                 | Rule                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A. Synonymous duplication | The same behavior appears in at least two places (aliases / conversions / stage wiring / error mapping) → violation; it must be promoted to a shared layer                                 |
| B. Excessive fan-out      | A single change requires synchronously modifying at least three existing synonymous mappings → **classified as an architectural change**; stop work and follow the design-document process |
| C. New entry point        | A “Nth entry / table item” is added instead of being registered in the declarative stage table → violation                                                                                 |

## Part Two: Decision Procedure (D0–D4)

Every code change must pass five gates in sequence. Each gate is a decidable Boolean condition;
**stop as soon as one is triggered**:

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

**Trade-off criteria (applicable throughout D0–D4)**: A proposal may be rejected **only** on grounds
of **correctness** and **readability**. The “size of the change,” “whether it introduces a new
mechanism,” and “implementation cost” **do not constitute grounds for rejection**. This operates in
parallel with Prohibition One: Prohibition One constrains whether concepts are duplicated, while
this criterion constrains whether the reasons for a trade-off are valid. A proposal rejected because
of its change footprint or because it introduces a new mechanism is sent straight back during
review.

When a trade-off requires a user decision—architectural choice, semantic deviation, whether to
register something, breaking change, or priority ordering—provide all four items required by “When
requesting user adjudication (mandatory)” in the repository-root `AGENTS.md`: situation and
evidence, item-by-item advantages and disadvantages, an explicit recommendation and its cost, and an
explicit statement of any breaking changes.

**Routing for semantic corrections (RFC-039 D59)**: All D0–D4 gates remain in force, with one
additional “error type” label. **Types A/B/C**—cases where the old behavior was **unsuccessful**:
crash or stack overflow, silent failure, silent mismatch, silent no-op, or silently incorrect
answer—may be corrected alongside a refactor, provided that the result goes **from red to green**,
the externally observable behavior of programs that **previously succeeded** does not change, the
correction is registered in the same commit, the baseline is regenerated in the same commit, and a
human review is performed. **Types D/E/F**—changes to the externally observable behavior of programs
that previously succeeded, external contract tables, or RFC-level design changes—require a user
decision first. See
[07 §Semantic-correction routing criteria](architecture/07-equivalence-oracle.md) and D59 in the
RFC-039 decision record for the detailed criteria; add a line for “Error type (A–F)” to the required
PR block.

## Part Three: Red Lines (Changes Must Be Rejected During Review)

1. **Do not change code you have not inspected**—you may not modify a file unless you have opened it
   and grepped for relevant references
2. **Do not leave core functionality as deferred implementation**—do not add silent-discard patterns
   such as `todo!()`, `unimplemented!()`, or `Vec::new() // Not implemented yet`, nor defer work
   indefinitely to a “standalone issue.” Fix it upon discovery, or register it as a task for the
   current phase
3. **Do not delete tests or relax criteria to obtain a green build**—being unable to satisfy them is
   an implementation defect; report it accurately and reassess. **Test admission and retirement
   follow Part Seven**: a test may be retired only if it fails all three gates (clause / incident
   history / proof); “delete it because it is failing” is always rejected
4. **Do not use import aliases to bridge semantically equivalent concepts**
5. **Do not add `include!` or new reverse cross-layer dependencies; cross-layer `pub(crate)`
   exposure may only decrease, never increase**
6. **The required PR block must be completed accurately**—if the authoritative source module for D0
   cannot be named, the source was not investigated and the change is rejected; a missing “Error
   type (A–F, per D59)” label, or one that does not match the actual change, is likewise rejected

## Part Four: Division of Labor Between Machines and Humans

**Assign mechanical issues to tools; review should focus only on non-mechanical issues.**

| Category                                                                                                                 | Executor                                                                   |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------- |
| Formatting, linting, table consistency, boundary erosion (Prohibition Two C), test wiring, cross-layer dependencies      | Tools: `cargo fmt` / `clippy` / `build.rs` gates / `scripts/ci/check-*.py` |
| Responsibility-category judgment (Prohibition Two A), D3’s “patch or architectural change” decision, exemption rationale | Human: required PR block + review                                          |

**Write corrections directly into the text; do not retain old/new comparisons**: when documentation
or comments do not match reality, directly change the text to state the facts and delete the old
wording. Do not retain the old text and append “revision notes” or “originally…, now…” comparisons;
readers should not have to encounter the incorrect version before the correct one. Evidence of the
**correction process** (red→green, before/after diff, adjudication number) belongs in the commit
message or the WBS historical record (`09a`), not in the body of the corrected text. The historical
record (`09a`) and work orders/change lists (such as `02` §Change List) are exempt from this
restriction.

## Part Five: Exemption Mechanism

- Exemptions from machine-enforced criteria must always use an inline `// reason: <reason>` comment
- The exemption count itself is included in the gate report, and each exemption is reviewed
  individually
- An exemption is **evidence of an exception**, not **a convention**; when the same type of
  exemption appears for the second time, the correct action is to change the rule or criterion, not
  to add a third exemption

## Part Six: Code Review Checklist

Review each PR against the following items:

- [ ] **D0**: Does the change touch any table? Which module is the sole authoritative implementation
      of that table?
- [ ] **D1**: Can the new concept be expressed using existing concepts and parameters? If not, where
      is the rationale recorded?
- [ ] **D2**: Is it semantically equivalent to an existing concept? Where is the conversion? Is an
      import alias required?
- [ ] **D3**: Does the same behavior appear in at least two places? Was an entry point added instead
      of registering it in the stage table? Must at least three synonymous mappings be changed?
- [ ] **D4**: Does the new code belong to the target module’s existing responsibility category?
- [ ] **Silent channel**: For a registration-to-consumption mechanism—one side registers data and
      the other consumes it, such as SemanticDB, proof_calls, or the obligation ledger—are the
      direct-path tests for the producer and consumer delivered in the same batch? If the consumer
      test manually constructs registered data while the producer has no direct-path evidence, the
      mechanism exists only on the surface and the data flow has never occurred; reject the change
      (three precedents: proof_calls, RFC-039 D54, and RFC-039 D55)
- [ ] **Boundaries**: Have they been eroded? Have cross-layer dependencies, `include!`, or
      cross-layer `pub(crate)` exposure been added?
- [ ] **Check admission**: Which of the three gates in Part Seven does the new test/gate pass? Where
      is the rationale documented? (For a check that passes the **proof** gate, also answer: if it
      were replaced with a correct alternative implementation, would it fail?)
- [ ] **Division of labor**: Are mechanical issues covered by tools? Is review spending time only on
      non-mechanical issues?

## Part Seven: Admission and Retirement of Checks and Tests

**A green result proves nothing.** Checks have only negative epistemic value: what they prove is
that “on this occasion, it did not get this point wrong.” Whether a check is worth keeping depends
solely on whether it guards a **property that has been explicitly stated**. If that cannot be
answered, the check is not safety; it is a tax on people who follow the rules.

### 7.1 The Sole Validation: If It Were Replaced with a Correct Alternative Implementation, Would It Fail?

| Result | Meaning                                                                                                                 | Disposition                                      |
| ------ | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| Fails  | It freezes an **implementation choice** (rendered text, message wording, exact line numbers, internal structural shape) | Characterize it; **do not admit it to the gate** |
| Passes | It guards a **property**                                                                                                | Legitimate check                                 |

Any check that freezes the current output wholesale (full snapshots, byte-for-byte comparisons) will
necessarily fail when replaced with a correct implementation; by definition, it cannot pass this
gate.

Corollary: the atomic unit of a test is not “one test,” but **one falsifiable distinction**. Writing
the same code path, input class, and assertion intent N times adds zero incremental information; the
mere fact that “this repository has thousands of tests” constitutes no evidence.

### 7.2 Admission: Three Gates

Every new check must pass **at least one** of the following three gates, and the rationale must be
recorded in a comment at the top of the test file or in the commit message:

| Gate                 | Required evidence                                                                                                                                         |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Clause**           | Name the relevant clause of a SPEC / RFC                                                                                                                  |
| **Incident history** | Name a real failure: which change introduced it, what the defect was, and how it was exposed at the time                                                  |
| **Proof**            | ① What is the property? ② Why **must** it hold, independently of implementation choices? ③ Why is this test the least expensive observation point for it? |

The three gates are **not equivalent**: a clause is an external constraint and may be ambiguous;
incident history covers only the occurrence that actually happened; **proof covers the entire class
of properties**, and is therefore the most durable over time—its value lies not in catching a past
occurrence, but in blocking a class of violations.

The third gate must have teeth. Everyone can say, “This is an invariant,” but most things described
that way actually freeze the current implementation. All three gates share one validation (7.1): if
replacing the check with a correct alternative implementation makes it fail, return it to
characterization—it is not proof, but a freeze disguised as proof.

### 7.3 Retirement: Reversal of the Burden of Proof

Run existing checks through 7.2: **keep any that passes at least one gate; delete any that passes
none.** There is no “observation period,” “downgrade to advisory,” or “retain after marking as
characterization”—all three merely prolong something whose rationale cannot be explained.

- **Deletion is not irreversible**: history remains in git, and anything can be recovered by commit
  when needed. Precisely because it can be recovered, “delete first and retrieve it later if
  necessary” is preferable to “keep everything unconditionally”;
- Delete in batches, with one commit per batch and the ability to revert the batch as a whole;
- For each batch, record in [09a](architecture/09a-wbs-history.md) what was deleted, which gate was
  used to judge it, and whether alternative coverage exists.

### 7.4 Cost Discipline

Cost is unrelated to legitimacy. Cost must be measurable (using the repository’s existing `cost:`
resource-profile marker) and included in the gate budget. **“CI runs in parallel, so it does not
matter” must not be used as a justification**—parallelism saves wall-clock time, but it does not
save the time each person spends on local self-checks, and that is the cost paid every day.

### 7.5 Boundary with Red Line 3

Red Line 3 concerns **motivation** (changing criteria to make CI green), whereas this part concerns
**cost and benefit** (whether the check is worthwhile). The two do not conflict.

**Mandatory order of adjudication in a conflict**: ① Is it red now because the **implementation is
wrong**, or because the **criterion is wrong**? ② If this cannot be answered, do not delete it and
do not change the criterion; ③ If the **criterion is wrong**, retire the check under 7.3; do not
modify the assertion.
