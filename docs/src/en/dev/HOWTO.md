# Implementer's Manual (HOWTO)

> **This page is the unified entry point for all changes, not the rule body itself.** The single
> source of truth for rules is:
>
> - For code changes → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rule body) and [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md)
>   (D1–D52)
> - For doc changes → [docs-rules.md](docs-rules.md) (Diátaxis classification rules + wiki-style
>   language)
>
> This page does only two things: **mandatory pre-work self-check** and **"patch-style fix"
> determination**. If this page conflicts with the rule body, the rule body wins.

## Step 0: Determine What You're Changing

| Your change             | Read section  | Rule body                          | Pre-commit validation     |
| ----------------------- | ------------- | ---------------------------------- | ------------------------- |
| Code (`src/`, `tests/`) | Section I     | [coding-rules.md](coding-rules.md) | See end of Section I      |
| Docs (`docs/`)          | Section II    | [docs-rules.md](docs-rules.md)     | See end of Section II     |
| Both                    | Both sections | Obey both                          | Run both sets of commands |

Note: code changes often require synchronizing doc changes — error code tables, CLI arguments, and
user-visible behaviors are all asserted in the docs. Passing Section I's acceptance does not mean
you're done. First answer: "In which doc is this behavior described, and does it need to change
too?"

## I. Modifying Code

### Which Phase You're In

Construction master table: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First locate
your Level-3 task entry** (e.g., 4.2.7) and confirm: whether its prerequisites are complete, which
acceptance criterion level (C1–C6) it uses (criterion definitions in
[07](architecture/07-equivalence-oracle.md)), and whether it is marked "exclusive commit".

### Pre-work Self-Check (Go Through All of These Before Writing Code)

- [ ] I have read the full Level-3 entry in [09](architecture/09-execution-wbs.md), not just the
      task title
- [ ] I have read the **actual context** of the code being changed (opened the file and looked, not
      imagined from the task description)
- [ ] I know the acceptance criterion level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavioral equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have run **D0**: Does this change touch any table (error code / opcode / type / phase)?
      Which module is the authoritative source? — If you can't write the module name, you haven't
      checked
- [ ] I have run **D1/D2**: Can the newly added type / enum / constant table be expressed using
      existing concepts plus parameters? Is it semantically equivalent to an existing concept? (Same
      semantics forbids bridging with `import` aliases)
- [ ] I have run **D4**: Does the new code belong to an **existing responsibility category** of the
      target module? (Authoritative definition of responsibility categories in
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] Is my new mechanism of the "register-consumer" type (one side registers data, the other
      consumes)? If so, are the **directly driven** end-to-end tests on the producer and consumer
      sides delivered in the same batch? Hand-crafted registered data in consumer tests = a silent
      channel; review rejects it (coding-rules Part 6; precedent: RFC-039 Decision Register D55)
- [ ] If this task writes/changes tests: I have read [test-specification.md](test-specification.md)
      — `test_<what>_<scenario>` naming prefix, AAA three-section comments over 5 lines, assertions
      with custom messages, enum matching via `assert!(matches!(...))`, no `use super::*`, no inline
      `mod tests {}`
- [ ] If the task says "red-first, then green" (e.g., 2.4.1, 8.8.1), I have confirmed the red
      criterion already exists and is currently red
- [ ] I have not prepared to "delete failing tests" or "loosen criteria" to pass acceptance —
      **inability to do it is an implementation defect; report it as-is, not a reason to loosen**
      (D24 / no C5′)
- [ ] I have asked myself: in which doc is this behavior described, and does this change require
      synchronizing doc changes (change → Section II)

### Is This a "Patch-Style Fix" (D3, Stop on Hit)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Item 3 is the reason this manual exists.** "Changing ≥3 synonymous mappings" means you are
maintaining a parallel source of truth — the correct action is to eliminate the parallelism, not to
add another one. Historical incidents: 3 sets of operator enums, 5 compile entry points,
hand-written type synonym tables (see [08](architecture/08-maintenance-mechanism.md) incident
table).

### A Few Hard Rules That Are Easy to Trip On

- **Don't modify code you haven't read**: If you haven't opened the file or grepped its references,
  you must not touch it. Use current repository line numbers — doc line numbers drift.
- **No unimplemented leftovers**: Core features must not include `todo!()` /
  `Vec::new() // Not implemented yet` / "separate issue to be fixed later". If you find one, fix it,
  or register it into the current phase's tasks — no new leftovers allowed (D52 precedent: the `.42`
  three-place data loss has all been folded into P7).
- **Verification items must be verified line by line**: When a task is marked "must verify line by
  line, cannot assume equivalence" (e.g., 4.2.7, 6.1.3), write the verification conclusion into the
  PR description.
- **Deleting tests to get a green light = violation**: DoD requires the test count at the end of
  each phase to be no lower than at the start.
- **Don't manually check what tools already cover**: fmt / clippy / `build.rs` gates /
  `scripts/ci/check-*.py` from P0 onward. Review only looks at what machines can't check
  (responsibility determination, D3).

### What to Do When You're Stuck

| Situation                              | Correct action                                                                                                                                                    | Wrong action                                               |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Criterion doesn't go green             | Report as-is, fix the implementation; if you suspect the criterion is wrong, go back to [07](architecture/07-equivalence-oracle.md) to propose a criterion change | Add a whitelist/exemption for the criterion                |
| Design premise turns out to be invalid | Go back to the [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md) to change the decision and explain why                                   | Quietly work around the decision                           |
| Task is larger than expected           | Break down the Level-3 task further in [09](architecture/09-execution-wbs.md)                                                                                     | Mix multiple acceptance criteria into one oversized commit |
| Doc contradicts code                   | Trust the code first, then fix the doc (precedent: `layers/README.md` had the layer order reversed, D7)                                                           | Write code that follows the wrong doc                      |

## II. Modifying Docs

### Classification: Which Directory Does This Doc Belong To

Use the decision tree from [docs-rules.md](docs-rules.md) §1 to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this
doc serves two audiences — split it first, then act.**

### Pre-work Self-Check (Go Through All of These Before Modifying Docs)

- [ ] I used the docs-rules §1 decision tree to determine the directory, and the answer is unique
- [ ] I grepped for similar content; the same content has no ≥2 copies (if it does, merge them; do
      not add a third)
- [ ] I checked `docs/glossary.json` and existing docs, reused existing terminology, and did not
      coin new names
- [ ] New doc: frontmatter `title` is set; sidebar is hooked up (`config.js` + `i18n/en.json`;
      `generateSidebar` scans the directory's files and needs no manual hookup)
- [ ] Moved/renamed doc: all six syncs completed (see "Six-Sync for Moving Docs" below)
- [ ] Language style has passed docs-rules §2: active voice, declarative statements, present tense,
      consistent terminology, no marketing words

### Six-Sync for Moving Docs (Moving = Changing URL, All Required)

1. Move with `git mv` (preserve file history)
2. Site-wide links: grep the old path and update each one (the build's dead-link check is a
   fallback, not a substitute)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. `en/` mirror: move the corresponding English file in sync
5. Scripts and configs that reference the path: `scripts/`, `docs/scripts/`, `.github/` workflows,
   `AGENTS.md`, `README.md`
6. The cache key in `docs/src/.i18n-cache.json` (key contains the file path)

### Is This a "Patch-Style Fix" (Doc-Version D3, Stop on Hit)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit Validation (Doc Changes)

```bash
cd docs
pnpm docs:build                                # 死链零容忍，构建必须绿
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # 文档示例真实执行
python ../scripts/ci/check-docs-orphan.py      # 孤儿页与重复 H1
python ../scripts/ci/check-docs-truth.py       # 版本注入与事实对账
```

If `rfc/` is changed, add one more:

```bash
python ../scripts/rfc/check_tracking.py        # RFC 状态一致性
```

## References

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, review checklist
  (**code rule body**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language (**doc rule
  body**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical incident
  table, external references, rejected-proposal rationales (diagnostic record, not a rule source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies,
  parallel groupings
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — master plan and decision register D1–D52
