# Implementer's Manual (Compiler Architecture Refactor)

> **This page is an entry point, not the rule body itself.** The sole authority for rules is in
> [coding-rules.md](coding-rules.md) (Three Prohibitions + D0–D4, the precise rule body)
> and [RFC-039 Decision Registry](../rfc/draft/039-compiler-architecture.md) (D1–D52). This page
> does only two things: **mandatory pre-work self-check** and **"patch-style fix" determination**.
> If this page conflicts with coding-rules/RFC, coding-rules/RFC prevails.

## Which Stage You're In

Construction master table: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find your Level-3
task entry** (e.g., 4.2.7) and confirm: whether its prerequisites are complete, which level of
acceptance criteria it uses (C1–C6) (criteria definitions in [07](architecture/07-equivalence-oracle.md)), and
whether it is marked as "exclusive commit".

## Pre-work Self-Check (Go Through All of Them Before Writing Code)

- [ ] I have read the original text of this task's Level-3 entry in [09](architecture/09-execution-wbs.md), not
      just looked at the task title
- [ ] I have read the **actual context** of the code being modified (opened the file and looked, not
      imagined based on task description)
- [ ] I know this task's acceptance criteria level (C1 zero-diff / C2 diagnostic set / C3 diagnostic
      code / C4 behavioral equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have run **D0**: Does this change touch any table (error code/opcode/type/phase)? Which is
      the authoritative source module? — If you can't write out the module name, you haven't looked
      it up
- [ ] I have run **D1/D2**: Can the new type/enum/constant table be expressed using existing
      concepts + parameters? Is it semantically identical to existing concepts? (Same semantics —
      bridging via import alias is forbidden)
- [ ] I have run **D4**: Does the new code belong to the **existing responsibility category** of the
      target module? (Authoritative definition of responsibility categories in
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] If the task says "red first, then green" (e.g., 2.4.1, 8.8.1), I confirm that the red criteria
      already exist and is currently actually red
- [ ] I have not prepared to "delete failing tests" or "loosen criteria" in order to pass acceptance
      — **inability to do so is an implementation defect; report it truthfully, not a reason to
      loosen** (D24 / no C5′)

## Is This a "Patch-Style Fix" (D3, Stop on Hit)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Item 3 is the reason this manual exists.** "Changing ≥3 synonymous mappings" means you're
maintaining a parallel set of facts — the correct action is to eliminate the parallelism, not add
another. Historical incidents: 3 sets of operator enums, 5 compilation entry points, hand-written
type synonym table (see [08](architecture/08-maintenance-mechanism.md) incident table).

## A Few Hard Rules That Are Easy to Step On

- **Don't modify code you haven't looked at**: Without opening the file or grepping reference
  points, you are not allowed to modify it. Line numbers follow the repository's current state;
  document line numbers drift.
- **Don't leave unimplemented leftovers**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / "track in a separate issue and fix later". If you find them,
  fix them, or register them into the current phase's tasks — don't introduce new leftovers (D52
  precedent: `.42` data loss in three places all incorporated into P7).
- **Verification items must be verified line by line**: When a task is marked "must be verified line
  by line, equivalent cannot be assumed" (e.g., 4.2.7, 6.1.3), include the verification conclusion
  in the PR description.
- **Deleting tests to get a green light = violation**: DoD requires that the test count at the end
  of each phase not be less than at the start.
- **Don't manually check what tools already cover**: fmt / clippy / `build.rs` gates /
  `scripts/ci/check-*.py` from P0 onward. Review only looks at what machines can't check
  (responsibility judgment, D3).

## What to Do If You're Stuck

| Situation                        | Correct Action                                                                                                                                | Wrong Action                                             |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Criteria won't pass              | Report truthfully, fix the implementation; if you suspect the criteria is wrong, return to [07](architecture/07-equivalence-oracle.md) to propose a change | Add a whitelist/exception to the criteria                |
| Design premise found to be false | Return to [RFC-039 Decision Registry](../rfc/draft/039-compiler-architecture.md) to change the decision and explain why                       | Bypass the decision and do it quietly                    |
| Task is bigger than expected     | Further break down the Level-3 task in [09](architecture/09-execution-wbs.md)                                                                              | One oversized commit mixing multiple acceptance criteria |
| Documentation contradicts code   | Trust the code first, then fix the documentation (precedent: `layers/README.md` layer order was reversed, D7)                                 | Write code following the wrong documentation             |

## References

- [coding-rules.md](coding-rules.md) — Three Prohibitions, D0–D4, red lines, review
  checklist (**rule body**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident table, external
  references, rejected scheme justifications (diagnostic record, not a rule source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies, parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criteria definitions
- [01-routing.md](architecture/01-routing.md) — Authoritative definition of responsibility categories, target
  directory structure
- [RFC-039](../rfc/draft/039-compiler-architecture.md) — Master plan and Decision Registry D1–D52
