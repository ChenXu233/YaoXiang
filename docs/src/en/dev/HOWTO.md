# Implementer's Handbook (HOWTO)

> **This page is the unified entry point for all changes, not the rule body itself.** The sole
> authority for the rules:
>
> - For code changes → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rule body) and [RFC-039 decision register](../rfc/accepted/039-compiler-architecture.md)
>   (D1–D52)
> - For documentation changes → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page only does two things: **mandatory pre-work self-checks** and **judging "patch-style
> fixes"**. If this page conflicts with the rule body, the rule body takes precedence.

## Step 0: Determine What You're Changing

| Your change                    | Which section to read | Rule body                          | Pre-commit verification   |
| ------------------------------ | --------------------- | ---------------------------------- | ------------------------- |
| Change code (`src/`, `tests/`) | Section 1             | [coding-rules.md](coding-rules.md) | See end of Section 1      |
| Change docs (`docs/`)          | Section 2             | [docs-rules.md](docs-rules.md)     | See end of Section 2      |
| Change both                    | Read both sections    | Obey both                          | Run both sets of commands |

Note: changing code often requires syncing the documentation—error code tables, CLI arguments, and
user-visible behavior all have assertions in the docs. Passing Section 1 acceptance does not mean
you're done; first answer "where is this behavior described in the docs, and does this change need
to update the docs along with it."

## 1. Changing Code

### Which Phase You're In

Master construction schedule: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find
your level-3 task entry** (e.g., 4.2.7), and confirm: are its prerequisites complete, what level of
acceptance criterion is it (C1–C6, criterion definitions see
[07](architecture/07-equivalence-oracle.md)), and whether it is marked "exclusive commit".

### Pre-work Self-check (Go through all before writing code)

- [ ] I have read the **original text** of this task's level-3 entry in
      [09](architecture/09-execution-wbs.md), not just the task title
- [ ] I have read the **actual context** of the code being changed (opened the file and looked, not
      imagined from the task description)
- [ ] I know the acceptance criterion level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavior equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have marked the **error type (A–F, per RFC-039 D59)**: Type A/B/C (old behavior **not
      successful**—crash/stack overflow, silent failure, silent mismatch, silent wrong answer) can
      be fixed as a side benefit of refactoring, but must be **red-then-green** and must not change
      the external behavior of "previously successful" programs; Type D/E/F (user-visible semantics
      / external contract table / RFC-level design) must first undergo user adjudication (criterion
      see [07 §Semantic Modification Triage Criterion](architecture/07-equivalence-oracle.md))
- [ ] I ran **D0**: Does this change touch any table (error code / opcode / type / phase)? What is
      the authoritative source module? — If you can't write the module name, you haven't checked
- [ ] I ran **D1/D2**: Can the newly added types/enums/constants table be expressed using existing
      concepts + parameters? Is it semantically the same as an existing concept? (Same-semantic is
      forbidden to bridge with import aliases)
- [ ] I ran **D4**: Does the new code belong to an **existing responsibility category** of the
      target module? (Authoritative definition of responsibility categories see
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] I ran the **trade-off criterion**: Is the rejection reason of me (or the proposal submitter)
      based solely on **correctness** and **readability**? Rejecting a proposal on the grounds of
      "large change surface / need to introduce new mechanisms / high cost" = invalid rejection,
      review will bounce it (coding-rules Part 2); trade-offs that need user adjudication shall
      provide all four items per repository root `AGENTS.md`
- [ ] Is my new mechanism "register-consume" type (one side registers data, the other consumes)? If
      so, has the **direct-drive** end-to-end test from producer to consumer been delivered in the
      same batch? Consumer-side tests that manually construct registration data = silent channel,
      review will bounce it (coding-rules Part 6; precedent see RFC-039 decision register D55)
- [ ] If this task requires writing/changing tests: I have read
      [test-specification.md](test-specification.md)—naming `test_<what>_<scenario>` prefix, more
      than 5 lines with AAA three-section comments, assertions with custom messages, enum matching
      `assert!(matches!(...))`, no `use super::*`, no inline `mod tests {}`
- [ ] If my change will alter IR or corpus execution behavior (frontend/ir_gen/std/corpus `.yx`): I
      plan to **commit in the same submission** update IR snapshots and corpus diff baseline
      (commands see AGENTS.md common commands section), and manually review both diffs—parallel
      stream #385 once missed syncing snapshots and was blocked by the gate (precedent dabdfd96)
- [ ] If the task says "red-then-green" (e.g., 2.4.1, 8.8.1), I confirm the red criterion exists and
      currently is indeed red
- [ ] I have not prepared "delete failing tests" or "loosen criteria" to pass acceptance—**inability
      to do so is an implementation defect, report truthfully, not a reason to loosen**
      (D24/non-existent C5')
- [ ] I have asked myself: where in the docs is this behavior described, does this change need to
      sync the docs (change → Section 2)

### Is this a "patch-style fix" (D3, hit and stop)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Item 3 is why this handbook exists.** "Changing ≥3 places of synonymous mapping" means you are
maintaining a parallel source of truth—the correct action is to eliminate the parallel, not add
another. Historical incidents: 3 sets of operator enums, 5 compilation entry points, hand-written
type synonym table (see [08](architecture/08-maintenance-mechanism.md) incident table for details).

### A Few Easy-to-Trip Hard Rules

- **Don't change code you haven't read**: Without opening the file, without grepping reference
  points, you must not touch it. Line numbers are subject to the repository as-is; documentation
  line numbers will drift.
- **No leftover unimplemented**: Core features must not have `todo!()` /
  `Vec::new() // Not implemented yet` / "independent issue to be fixed later". If found, fix it, or
  register it in the current phase task—no new legacy allowed (D52 precedent: `.42` three data loss
  points all absorbed into P7).
- **Verification items must be verified line by line**: When the task notes "must verify line by
  line, cannot assume equivalence" (e.g., 4.2.7, 6.1.3), write the verification conclusion into the
  PR description.
- **Deleting tests for a green light = violation**: DoD requires that the test count at the end of
  each phase is not less than at the start.
- **Don't manually check what tools have already caught**: fmt / clippy / `build.rs` gate / P0
  onward `scripts/ci/check-*.py`. Review only looks at what machines can't check (responsibility
  judgment, D3).

### What to Do When Stuck

| Situation                            | Correct Action                                                                                                                                                       | Wrong Action                                        |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Criterion won't go green             | Report truthfully, fix the implementation; suspect the criterion is wrong, go back to [07](architecture/07-equivalence-oracle.md) and propose changing the criterion | Add whitelist/exemption to the criterion            |
| Discover design premise doesn't hold | Go back to [RFC-039 decision register](../rfc/accepted/039-compiler-architecture.md) and change the decision with explanation                                        | Bypass the decision and do it quietly               |
| Task is bigger than expected         | In [09](architecture/09-execution-wbs.md), further break down the level-3 task                                                                                       | One huge commit mixing multiple acceptance criteria |
| Discover docs and code conflict      | Trust the code first, then fix the docs (precedent: `layers/README.md` layer order was reversed, D7)                                                                 | Write code following the wrong docs                 |

## 2. Changing Documentation

### Classification: Which directory does this document belong to?

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **Cannot produce a unique answer means
this document serves two audiences—split it first, then act.**

### Pre-work Self-check (Go through all before changing docs)

- [ ] I have used the docs-rules §1 decision tree to determine the directory, the answer is unique
- [ ] I have grepped for similar content; the same content has no ≥2 copies (if so, merge, don't add
      a 3rd)
- [ ] I have checked docs/glossary.json and existing docs, used existing terminology, did not coin
      new names
- [ ] New document: frontmatter `title` is written; sidebar is mounted (`config.js` +
      `i18n/en.json`; generateSidebar scans files in the directory and does not require manual
      mounting)
- [ ] Move/rename document: all six syncs completed (see "Six syncs for moving documents" below)
- [ ] Language style has passed docs-rules §2: active voice, definitive statements, present tense,
      consistent terminology, no marketing words

### Six Syncs for Moving Documents (moving = changing URL, all required)

1. Use `git mv` to move (preserve file history)
2. Site-wide links: grep old path and update one by one (build's dead link check will catch it, but
   won't do it for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: sync move the corresponding English file
5. Scripts and configs that reference the path: `scripts/`, `docs/scripts/`, `.github/` workflows,
   `AGENTS.md`, `README.md`
6. `docs/src/.i18n-cache.json` cache keys (keys contain file paths)

### Is this a "patch-style fix" (Documentation version D3, hit and stop)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit Verification (changing docs)

```bash
cd docs
pnpm docs:build                                # 死链零容忍，构建必须绿
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # 文档示例真实执行
python ../scripts/ci/check-docs-orphan.py      # 孤儿页与重复 H1
python ../scripts/ci/check-docs-truth.py       # 版本注入与事实对账
```

If you changed `rfc/`, add one more:

```bash
python ../scripts/rfc/check_tracking.py        # RFC 状态一致性
```

## Reference

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, review checklist
  (**code rule body**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language (**doc rule
  body**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical incident
  table, external references, rejected proposal arguments (diagnostic records, not rule sources)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — level-3 task table, dependencies,
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — master plan and decision register D1–D52
