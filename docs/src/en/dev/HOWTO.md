# Implementer's Manual (HOWTO)

> **This page is the unified entry point for all changes, not the rules themselves.** The sole
> authority of the rules:
>
> - For code changes → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rules) and [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - For documentation changes → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory self-check before starting work** and **"patch-style
> fix" determination**. If this page conflicts with the rules, the rules take precedence.

## Step 0: Determine what you are changing

| Your change                    | Read section  | Rules                              | Pre-commit validation     |
| ------------------------------ | ------------- | ---------------------------------- | ------------------------- |
| Change code (`src/`, `tests/`) | Section 1     | [coding-rules.md](coding-rules.md) | See end of Section 1      |
| Change documentation (`docs/`) | Section 2     | [docs-rules.md](docs-rules.md)     | See end of Section 2      |
| Both                           | Both sections | Follow both                        | Run both sets of commands |

Note: Code changes often require simultaneous documentation changes — error code tables, CLI
parameters, user-visible behaviors are all asserted in documentation. Passing Section 1's acceptance
does not mean it's done; first answer "where is this behavior described in documentation, does it
need to be updated."

## I. Changing Code

### Which stage are you in

Master schedule: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find your level-3
task entry** (e.g., 4.2.7), confirm its: prerequisites completed, acceptance criteria level (C1–C6,
criteria definitions in [07](architecture/07-equivalence-oracle.md)), whether marked as "exclusive
commit".

### Pre-work self-check (all must pass, before writing code)

- [ ] I have read the original text of this task's level-3 entry in
      [09](architecture/09-execution-wbs.md), not just the task title
- [ ] I have read the **actual context** of the code being changed (opened the file to look, not
      just imagined from the task description)
- [ ] I know the acceptance criteria level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavioral equivalence / C5 AST+diagnostic+behavior / C6 pure deletion)
- [ ] I have run **D0**: Does this change touch any table (error code/opcode/type/phase)? What is
      the authoritative source module? — If you cannot name the module, you have not checked
- [ ] I have run **D1/D2**: Can the new type/enum/constant table be expressed using existing
      concepts + parameters? Is it semantically the same as existing concepts? (Same semantics
      forbids bridging with import aliases)
- [ ] I have run **D4**: Does the new code belong to a **pre-existing responsibility category** of
      the target module? (Authoritative definition of responsibility categories in
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] I have run **trade-off criteria**: Are my (or the proposal author's) rejection reasons based
      only on **correctness** and **readability**? Rejecting a solution with reasons like "large
      change area/needs new mechanism/high cost" = invalid rejection, review bounces (coding-rules
      Part 2); trade-offs requiring user decision must follow [AGENTS.md](../../../AGENTS.md) for
      all four items
- [ ] Is my new mechanism "register-consume" type (one party registers data, the other consumes)? If
      so, are the **direct-driven** end-to-end tests for producer and consumer delivered in the same
      batch? Consumer-side tests manually constructing registration data = silent channel, review
      bounces (coding-rules Part 6; precedent see RFC-039 Decision Registry D55)
- [ ] If this task requires writing/changing tests: I have read
      [test-specification.md](test-specification.md) — `test_<what>_<scenario>` naming prefix, AAA
      three-section comments over 5 lines, assertions with custom messages, enum matching
      `assert!(matches!(...))`, no `use super::*`, no inline `mod tests {}`
- [ ] If my changes will alter IR or corpus runtime behavior (frontend/ir_gen/std/corpus `.yx`): I
      plan to **update IR snapshots and corpus diff baselines in the same commit** (commands in
      AGENTS.md common commands section), and manually review both diffs — parallel stream #385 once
      missed syncing snapshots and was blocked by the gate (precedent dabdfd96)
- [ ] If the task says "red before green" (e.g., 2.4.1, 8.8.1), I confirm the red criteria exist and
      it is indeed red now
- [ ] I have not prepared to "delete failing tests" or "loosen criteria" to pass acceptance —
      **inability to do so is an implementation defect, report honestly, not a reason to loosen**
      (D24/no C5′)
- [ ] I have asked myself: where is this behavior described in documentation, does this change
      require updating documentation accordingly (change → Section 2)

### Is this a "patch-style fix" (D3, stop if hit)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Item 3 is why this manual exists.** "Changing ≥3 synonymous mappings" means you are maintaining a
parallel truth — the correct action is to eliminate the parallel, not add another. Historical
accidents: 3 sets of operator enums, 5 compilation entry points, hand-written type synonym table
(see [08](architecture/08-maintenance-mechanism.md) accident table for details).

### Some hard rules that are easy to step on

- **Don't change code you haven't read**: Without opening the file or grepping references, you
  cannot change it. Line numbers are based on the current state of the repository, document line
  numbers will drift.
- **No TODOs left behind**: Core functionality cannot have `todo!()` /
  `Vec::new() // Not implemented yet` / "separate issue to fix later". If found, fix it, or register
  it in the current phase's tasks — do not bring out new TODOs (D52 precedent: `.42` three data loss
  cases all collected into P7).
- **Verification items must be verified line by line**: When the task notes "must verify line by
  line, cannot assume equivalence" (e.g., 4.2.7, 6.1.3), write the verification conclusion into the
  PR description.
- **Deleting tests for green = violation**: DoD requires that the number of tests at the end of each
  phase is not less than at the start.
- **Don't manually check what tools have caught**: fmt / clippy / `build.rs` gate /
  `scripts/ci/check-*.py` from P0. Review only looks at what machines can't check (responsibility
  determination, D3).

### What to do when stuck

| Situation                            | Correct action                                                                                                                                          | Wrong action                                             |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Criteria don't pass green            | Report honestly, fix implementation; suspect criteria is wrong then go back to [07](architecture/07-equivalence-oracle.md) to propose changing criteria | Add whitelist/exemption to criteria                      |
| Found design premise doesn't hold    | Go back to [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md) to change decision and explain why                                 | Circumvent decision and do it quietly                    |
| Task is bigger than expected         | Break down the level-3 task further in [09](architecture/09-execution-wbs.md)                                                                           | One oversized commit mixing multiple acceptance criteria |
| Found documentation contradicts code | First trust the code, then fix the documentation (precedent: `layers/README.md` layer order was reversed, D7)                                           | Write code according to wrong documentation              |

## II. Changing Documentation

### Classification: Which directory does this document belong to

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, it
means this document serves two audiences — split first, then act.**

### Pre-work self-check (all must pass, before changing documentation)

- [ ] I used docs-rules §1 decision tree to determine the directory, the answer is unique
- [ ] I grepped for similar content; no ≥2 copies of the same content exist (if so, merge, do not
      add a third)
- [ ] I checked docs/glossary.json and existing documentation, used existing terminology, didn't
      create new names
- [ ] New documentation: frontmatter `title` is written; sidebar is attached (`config.js` +
      `i18n/en.json`; generateSidebar scanning files in directory does not need manual attachment)
- [ ] Move/rename documentation: all six synchronizations completed (see "Six Synchronizations for
      Moving Documents" below)
- [ ] Language style has passed docs-rules §2: active voice, declarative statements, present tense,
      consistent terminology, no marketing words

### Six Synchronizations for Moving Documents (moving = changing URL, all required)

1. Use `git mv` to move (preserve file history)
2. Site-wide links: grep old path and update one by one (build's dead link check will catch it, but
   won't do it for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: synchronously move the corresponding English file
5. Scripts and configurations referencing that path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, `README.md`
6. `docs/src/.i18n-cache.json` cache key (key contains file path)

### Is this a "patch-style fix" (documentation version D3, stop if hit)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit validation (changing documentation)

```bash
cd docs
pnpm docs:build                                # Zero tolerance for dead links, build must be green
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Documentation examples actually executed
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
```

If `rfc/` was changed, add one more:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, review checklist
  (**code rules body**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language
  (**documentation rules body**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical accident
  table, external references, rejected solution arguments (diagnostic records, not rule sources)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies,
  parallel grouping
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criteria definitions
- [01-routing.md](architecture/01-routing.md) — Authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — Master plan and decision registry D1–D52
