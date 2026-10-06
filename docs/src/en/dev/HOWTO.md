# Implementer's Manual (HOWTO)

> **This page is the unified entry point for all changes, not the rule body itself.** The sole
> authority for rules is:
>
> - For code changes → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rule body) and [RFC-039 Decisions Log](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - For documentation changes → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory pre-work self-check** and **determining if a change is
> a "patch-style fix"**. If this page conflicts with the rule body, the rule body takes precedence.

## Step 0: Determine What You Need to Change

| Your change                    | Read which section | Rule body                          | Pre-commit verification   |
| ------------------------------ | ------------------ | ---------------------------------- | ------------------------- |
| Change code (`src/`, `tests/`) | Section 1          | [coding-rules.md](coding-rules.md) | See end of Section 1      |
| Change docs (`docs/`)          | Section 2          | [docs-rules.md](docs-rules.md)     | See end of Section 2      |
| Both                           | Both sections      | Both must be followed              | Run both sets of commands |

Note: Code changes often require synchronous documentation changes—error code tables, CLI
parameters, and user-visible behavior are all asserted in the documentation. Passing the Section 1
acceptance check is not the end; first answer "Which document describes this behavior, and does it
need to be updated accordingly?"

## 1. Changing Code

### What Stage You're In

Execution WBS: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find your level-3
task entry** (e.g., 4.2.7), and confirm: whether prerequisites are complete, which C1–C6 level of
acceptance criterion applies (criterion definitions in [07](architecture/07-equivalence-oracle.md)),
and whether it is marked as "exclusive commit".

### Pre-work self-check (go through all of these before writing code)

- [ ] I have read the original text of the level-3 task entry in
      [09](architecture/09-execution-wbs.md), not just the task title
- [ ] I have read the **actual context** of the code being changed (opened the file and looked, not
      imagined from the task description)
- [ ] I know the acceptance criterion level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavioral equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have run **D0**: Does this change touch any table (error code / opcode / type / phase)?
      Which is the authoritative source module? — If you can't write out the module name, you
      haven't checked
- [ ] I have run **D1/D2**: Can the newly added type/enum/constant table be expressed using existing
      concepts + parameters? Is it semantically the same as existing concepts? (Same semantics
      prohibits bridging with import aliases)
- [ ] I have run **D4**: Does the new code belong to the **existing responsibility category** of the
      target module? (Authoritative definition of responsibility categories in
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] If the task says "red-then-green" (e.g., 2.4.1, 8.8.1), I confirm the red criterion exists and
      is currently indeed red
- [ ] I have not prepared to "delete failing tests" or "relax criteria" just to pass acceptance —
      **Not being able to do it is an implementation defect, report truthfully, not a reason to
      relax** (D24 / no C5′)
- [ ] I have asked myself: Which document describes this behavior, and does this change need to
      update documentation accordingly (yes → Section 2)

### Is this a "patch-style fix"? (D3, stop if matched)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Item 3 is the reason this manual exists.** "Modifying ≥3 synonymous mappings" means you're
maintaining a parallel set of facts—the correct action is to eliminate the parallel, not add
another. Historical incidents: 3 sets of operator enums, 5 compile entry points, hand-written type
synonym tables (see [08](architecture/08-maintenance-mechanism.md) incident table).

### A Few Hard Rules That Are Easy to Step On

- **Don't change code you haven't read**: Without opening the file or grepping reference points, you
  must not modify it. Line numbers are based on the current state of the repository; documentation
  line numbers will drift.
- **No implementation leftovers**: Core functionality must not have `todo!()` /
  `Vec::new() // Not implemented yet` / "independent issue to be fixed later". If found, fix it, or
  record it in the current phase's tasks—do not introduce new leftovers (D52 precedent: `.42` three
  places of data loss have all been incorporated into P7).
- **Verification items must be verified line by line**: When a task is marked "must verify line by
  line, cannot assume equivalence" (e.g., 4.2.7, 6.1.3), write the verification conclusion in the PR
  description.
- **Deleting tests to get a green light = violation**: DoD requires that the test count at the end
  of each phase is not lower than at the start.
- **Don't manually check what tools have already covered**: fmt / clippy / `build.rs` gates / from
  P0 the `scripts/ci/check-*.py`. Reviews only look at things the machine can't check
  (responsibility determination, D3).

### What to Do When Stuck

| Situation                            | Correct Action                                                                                                                                                         | Wrong Action                                        |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Criterion doesn't pass green         | Report truthfully, fix implementation; if you suspect the criterion is wrong, go back to [07](architecture/07-equivalence-oracle.md) to propose changing the criterion | Add whitelist/exemption to the criterion            |
| Design premise found invalid         | Go back to [RFC-039 Decisions Log](../rfc/accepted/039-compiler-architecture.md) to change the decision and explain why                                                | Bypass the decision and do it secretly              |
| Task is larger than expected         | In [09](architecture/09-execution-wbs.md), break down the level-3 task further                                                                                         | One huge commit mixing multiple acceptance criteria |
| Found documentation contradicts code | Trust the code first, then fix the documentation (precedent: `layers/README.md` layer order was reversed, D7)                                                          | Write code following the wrong documentation        |

## 2. Changing Documentation

### Classification: Which Directory Does This Document Belong To

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer, the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you can't arrive at a unique answer,
this document is serving two audiences—split it first, then start work.**

### Pre-work self-check (go through all of these before changing documentation)

- [ ] I have used the docs-rules §1 decision tree to determine the directory, the answer is unique
- [ ] I have grepped similar content; the same content does not have ≥2 copies (if so, merge, don't
      add a 3rd)
- [ ] I have checked docs/glossary.json and existing documents, used existing terms, did not invent
      new names
- [ ] New document: frontmatter `title` is written; sidebar is mounted (`config.js` +
      `i18n/en.json`; generateSidebar scans files in the directory without manual mounting)
- [ ] Moved/renamed document: all six synchronizations complete (see "Moving a Document: Six
      Synchronizations" below)
- [ ] Language style has passed docs-rules §2: active voice, definitive statements, present tense,
      consistent terminology, no marketing words

### Moving a Document: Six Synchronizations (moving = changing URL, none can be missed)

1. Move with `git mv` (preserve file history)
2. Site-wide links: grep the old path and update one by one (build's dead link check will catch it,
   but won't do it for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: synchronously move the corresponding English file
5. Scripts and configurations that reference this path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, `README.md`
6. `docs/src/.i18n-cache.json` cache keys (key contains file path)

### Is this a "patch-style fix"? (documentation version of D3, stop if matched)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit Verification (Changing Documentation)

```bash
cd docs
pnpm docs:build                                # Zero tolerance for dead links, build must be green
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Document examples are actually executed
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
```

If you changed `rfc/`, add one more:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, review checklist
  (**Code Rule Body**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language
  (**Documentation Rule Body**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident
  table, external references, rejected plan rationale (diagnostic records, not rule source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies,
  parallel grouping
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — Authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — Master plan and decisions log D1–D52
