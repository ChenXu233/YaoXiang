# Implementer's Handbook (HOWTO)

> **This page is the unified entry point for all changes, not the rule body itself.** The sole
> authority for rules is:
>
> - Modifying code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rule body) and [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md)
>   (D1–D52)
> - Modifying documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory pre-work self-check** and **"patch-style fix"
> determination**. If this page conflicts with the rule body, the rule body takes precedence.

## Step 0: Determine What You're Changing

| Your change                    | Which section to read | Rule body                          | Pre-commit validation |
| ------------------------------ | --------------------- | ---------------------------------- | --------------------- |
| Modify code (`src/`, `tests/`) | Section 1             | [coding-rules.md](coding-rules.md) | See end of Section 1  |
| Modify documentation (`docs/`) | Section 2             | [docs-rules.md](docs-rules.md)     | See end of Section 2  |
| Both                           | Read both sections    | Follow both                        | Run both command sets |

Note: Modifying code often requires syncing the documentation — error code tables, CLI arguments,
and user-visible behavior are all asserted in the documentation. Passing Section 1's acceptance does
not mean you're done — first answer "where is this behavior described in the documentation, and does
it need to be updated accordingly."

## I. Modifying Code

### Which Stage Are You In

Construction master table: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First locate
your level-3 task entry** (e.g., 4.2.7), and confirm: whether its prerequisites are complete, which
acceptance criterion level it is (C1–C6) (criterion definitions see
[07](architecture/07-equivalence-oracle.md)), and whether it's marked as "exclusive commit."

### Pre-work Self-Check (Go Through All of These Before Writing Code)

- [ ] I have read the original text of my level-3 entry in [09](architecture/09-execution-wbs.md),
      not just the task title
- [ ] I have read the **actual context** of the code being changed (opened the file, not imagined
      based on the task description)
- [ ] I know the acceptance criterion level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavior equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have run **D0**: Does this change touch any table (error code / opcode / type / stage)? What
      is the authoritative source module? — If you can't write the module name, you haven't checked
- [ ] I have run **D1/D2**: Can the newly added type / enum / constant table be expressed using
      existing concepts + parameters? Is the semantics the same as existing concepts? (Same
      semantics cannot be bridged using import aliases)
- [ ] I have run **D4**: Does the new code belong to a **responsibility category already in the
      target module**? (Authoritative definition of responsibility categories see
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] I have run the **tradeoff criterion**: Is my (or the proposal author's) rejection reason based
      only on **correctness** and **readability**? Rejecting a proposal on the grounds of "large
      change scope / need to introduce a new mechanism / high cost" = invalid rejection, review
      kicks back (coding-rules part 2); tradeoffs requiring user decision provide all four items per
      the repo root `AGENTS.md`
- [ ] Is my new mechanism of "register-consume" type (one party registers data, the other consumes)?
      If so, have end-to-end tests that **directly drive** both producer and consumer been delivered
      in the same batch? Consumer tests that manually construct registered data = silent channel,
      review kicks back (coding-rules part 6; precedent see RFC-039 decision registry D55)
- [ ] If this task requires writing/modifying tests: I have read
      [test-specification.md](test-specification.md) — naming `test_<what>_<scenario>` prefix, AAA
      three-segment comment when exceeding 5 lines, assertions with custom messages, enum matching
      with `assert!(matches!(...))`, no `use super::*`, no inline `mod tests {}`
- [ ] If my change will alter IR or corpus runtime behavior (frontend / ir_gen / std / corpus
      `.yx`): I plan to **same commit** update IR snapshots and corpus diff baselines (commands see
      AGENTS.md common commands area), and manually review both diffs — parallel stream #385 once
      missed syncing snapshots and was blocked by the gate (precedent dabdfd96)
- [ ] If the task says "red first then green" (e.g., 2.4.1, 8.8.1), I confirm the red criterion
      already exists and is indeed red currently
- [ ] I have not prepared "deleting failing tests" or "loosening the criterion" to pass acceptance —
      **inability to do it is an implementation defect, report honestly, not a reason to loosen**
      (D24 / no such C5′)
- [ ] I have asked myself: where is this behavior described in the documentation, does this change
      require syncing the documentation (modifying → Section 2)

### Is This a "Patch-Style Fix" (D3, Stop on Hit)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Rule 3 is why this handbook exists.** "Changing ≥3 synonymous mappings" means you are maintaining
a parallel fact — the correct action is to eliminate the parallel, not to add another one.
Historical incidents: 3 sets of operator enums, 5 compilation entries, hand-written type synonym
table (see [08](architecture/08-maintenance-mechanism.md) incident table for details).

### Several Easy-to-Miss Hard Rules

- **Don't modify code you haven't read**: Without opening the file or grepping references, you must
  not modify it. Line numbers are based on the current state of the repo; doc line numbers will
  drift.
- **No "to be implemented" leftovers**: Core functionality must not have `todo!()` /
  `Vec::new() // Not implemented yet` / "separate issue to fix later". If found, fix it, or register
  it in the current phase's tasks — no new leftovers (D52 precedent: `.42` three data losses all
  incorporated into P7).
- **Verification items must be verified line by line**: When the task annotates "must verify line by
  line, cannot assume equivalence" (e.g., 4.2.7, 6.1.3), write the verification conclusion in the PR
  description.
- **Deleting tests for green light = violation**: DoD requires that the test count at the end of
  each phase is not less than at the start.
- **Mechanically-checked parts covered by tools don't need manual checks**: fmt / clippy /
  `build.rs` gate / `scripts/ci/check-*.py` from P0 onwards. Review only checks what machines can't
  (responsibility determination, D3).

### What to Do When Stuck

| Situation                               | Correct action                                                                                                                                                          | Incorrect action                                         |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| Acceptance criterion doesn't pass green | Report honestly, fix the implementation; if you suspect the criterion is wrong, return to [07](architecture/07-equivalence-oracle.md) to propose changing the criterion | Whitelist / exempt the criterion                         |
| Found a design premise doesn't hold     | Return to [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md) to change the decision and explain why                                              | Circumvent the decision and do it quietly                |
| Task is larger than expected            | In [09](architecture/09-execution-wbs.md), further split the level-3 task                                                                                               | One oversized commit mixing multiple acceptance criteria |
| Found documentation contradicts code    | Trust the code first, then fix the documentation (precedent: `layers/README.md` layer order was reversed, D7)                                                           | Write code according to the wrong documentation          |

## II. Modifying Documentation

### Classification: Which Directory Does This Document Belong To

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you can't give a unique answer, it
means this document serves two audiences — split it first, then take action.**

### Pre-work Self-Check (Go Through All of These Before Modifying Documentation)

- [ ] I used the docs-rules §1 decision tree to determine the directory; the answer is unique
- [ ] I grepped for similar content; the same content has no ≥2 duplicates (if so, merge, don't add
      a 3rd)
- [ ] I checked docs/glossary.json and existing documentation, follow existing terminology, didn't
      coin new names
- [ ] New document: frontmatter `title` is filled in; sidebar is linked (`config.js` +
      `i18n/en.json`; generateSidebar scans files in the directory without manual mounting)
- [ ] Move / rename document: all six syncs complete (see "Six-Sync for Moving Documents" below)
- [ ] Language style has passed docs-rules §2: active voice, declarative statements, present tense,
      consistent terminology, no marketing words

### Six-Sync for Moving Documents (Moving = Changing URL, All Required)

1. Use `git mv` to move (preserve file history)
2. Whole-site links: grep old paths and update one by one (the build's dead link check will fall
   back, but doesn't do it for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: sync move the corresponding English file
5. Scripts and configurations that reference the path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, `README.md`
6. `docs/src/.i18n-cache.json` cache key (key contains file path)

### Is This a "Patch-Style Fix" (Documentation Version of D3, Stop on Hit)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit Validation (Modifying Documentation)

```bash
cd docs
pnpm docs:build                                # zero tolerance for dead links, build must be green
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # actually execute doc examples
python ../scripts/ci/check-docs-orphan.py      # orphan pages and duplicate H1
python ../scripts/ci/check-docs-truth.py       # version injection and fact reconciliation
```

If you modified `rfc/`, add another one:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## References

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, review checklist
  (**code rule body**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language
  (**documentation rule body**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical incident
  table, external references, rejected proposal arguments (diagnostic record, not a rule source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — level-3 task table, dependencies,
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — master plan and decision registry D1–D52
