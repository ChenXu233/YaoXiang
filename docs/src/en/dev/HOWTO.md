# Implementer's Handbook (HOWTO)

> **This page is the single entry point for all changes; it is not the source of the rules.** The
> sole authorities for the rules are:
>
> - To change code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the exact
>   rules) and [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - To change documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory self-checks before work starts** and **determining
> whether a fix is a "patch-style fix."** If this page conflicts with the rulebooks, the rulebooks
> take precedence.

## Step 0: Determine What You Are Changing

| Your change                    | Which section to read | Source of truth                    | Validation before commit  |
| ------------------------------ | --------------------- | ---------------------------------- | ------------------------- |
| Change code (`src/`, `tests/`) | Section 1             | [coding-rules.md](coding-rules.md) | See the end of Section 1  |
| Change documentation (`docs/`) | Section 2             | [docs-rules.md](docs-rules.md)     | See the end of Section 2  |
| Change both                    | Read both sections    | Follow both rulebooks              | Run both sets of commands |

Note: Code changes often require synchronized documentation changes: error-code tables, CLI options,
and user-visible behavior are all asserted in the documentation. Passing Section 1's acceptance
checks does not mean you are done. First answer: "Where is this behavior documented, and does it
need to change with the code?"

## 1. Changing Code

### Which Stage Are You In

Execution work breakdown: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First, locate
your level-3 task item** (e.g., 4.2.7) and confirm: whether its prerequisites are complete, which
C1–C6 acceptance level applies (the criteria are defined in
[07](architecture/07-equivalence-oracle.md)), and whether it is marked as an "exclusive commit."

### Pre-Work Self-Check (Complete Every Item Before Writing Code)

- [ ] I have read the original text of this task's level-3 entry in
      [09](architecture/09-execution-wbs.md), not merely its title
- [ ] I have read the **actual context** of the code being changed (I opened the file rather than
      imagining it from the task description)
- [ ] I know this task's acceptance level (C1 zero-diff / C2 diagnostic set / C3 diagnostic codes /
      C4 behavioral equivalence / C5 AST + diagnostics + behavior / C6 pure deletion)
- [ ] I have classified the **error type (A–F, per RFC-039 D59)**: A/B/C (the old behavior **did not
      succeed**—crash/stack overflow, silent failure, silent mismatch, or silently producing the
      wrong answer) may be fixed along the way during refactoring, but the fix must be **red first,
      then green**, and must not change the externally observable behavior of programs that
      previously succeeded; D/E/F (user-visible semantics / external contract tables / RFC-level
      design) require user adjudication first (for the routing criteria, see
      [07 §Semantic-Correction Routing Criteria](architecture/07-equivalence-oracle.md))
- [ ] I have performed **D0**: Does this change touch any table (error code/opcode/type/stage)?
      Which module is the authoritative source? — If you cannot name the module, you have not
      investigated
- [ ] I have performed **D1/D2**: Can the newly added type/enum/constant table be expressed with
      existing concepts + parameters? Is it semantically identical to an existing concept? (For
      identical semantics, using import aliases to paper over the difference is prohibited)
- [ ] I have performed **D4**: Does the new code belong to an **existing responsibility category**
      of the target module? (For the authoritative definition of responsibility categories, see the
      directory responsibility table in [01-routing.md](architecture/01-routing.md))
- [ ] I have applied the **trade-off criterion**: Do my (or the proposal author's) reasons for
      rejecting a proposal rely **only on correctness and readability**? Rejecting a proposal
      because the change scope is large, it introduces a new mechanism, or it is costly is an
      invalid rejection, and review will send it back (coding-rules, Part 2); for trade-offs
      requiring a user decision, provide all four items as required by the repository-root
      `AGENTS.md`
- [ ] Is my new mechanism of the "register-and-consume" type (one party registers data and another
      consumes it)? If so, have the producer and consumer's **directly driven** end-to-end tests
      been delivered in the same batch? A consumer test that manually constructs registered data is
      a silent channel, and review will reject it (coding-rules, Part 6; precedent: RFC-039 Decision
      Registry D55)
- [ ] If this task involves writing or changing tests: I have read
      [test-specification.md](test-specification.md)—the `test_<what>_<scenario>` prefix, AAA
      three-part comments for tests over 5 lines, custom messages in assertions, enum matching with
      `assert!(matches!(...))`, no `use super::*`, and no inline `mod tests {}`
- [ ] If my change affects IR or corpus execution behavior (frontend/ir_gen/std/corpus `.yx`): I
      plan to update the IR snapshot and corpus diff baseline **in the same commit** (see the Common
      Commands section of AGENTS.md), and manually review both diffs—parallel workstream #385 once
      missed the snapshot update and was caught by the gate (precedent: dabdfd96)
- [ ] If the task says "red first, then green" (e.g., 2.4.1, 8.8.1), I have confirmed that the red
      criterion already exists and is currently red
- [ ] I have not prepared to "delete failing tests" or "relax the criterion" merely to pass
      acceptance—**inability to do so is an implementation defect; report it honestly rather than
      weakening the criterion** (D24/no C5′ exists)
- [ ] I have asked myself: Where is this behavior documented, and does this change require a
      synchronized documentation update (change → Section 2)

### Is This a "Patch-Style Fix"? (D3—If Matched, Stop)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Item 3 is the reason this handbook exists.** "Changing ≥3 synonymous mappings" means maintaining a
parallel source of truth—the correct action is to eliminate the parallel source, not add another
one. Historical incidents: three sets of operator enums, five compiler entry points, and a
hand-written table of type synonyms (see the incident table in
[08](architecture/08-maintenance-mechanism.md)).

### Hard Rules That Are Easy to Trip Over

- **Do not change code you have not inspected**: If you have not opened the file or grepped for
  references, you may not touch it. Use the line numbers in the current repository; line numbers in
  documentation drift.
- **Do not leave deferred implementation behind**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / "fix later in a separate issue." If you find one, fix it or
  record it as a task in the current phase—do not introduce new deferred work (D52 precedent: all
  three `.42` data-loss incidents have been brought under P7).
- **Verification items must be checked line by line**: When a task is marked "must be verified line
  by line; equivalence cannot be assumed" (e.g., 4.2.7, 6.1.3), write the verification conclusion in
  the PR description.
- **Deleting tests to get a green build is a violation**: The DoD requires that the number of tests
  at the end of each stage must not be lower than at the beginning.
- **Do not manually check mechanical issues already covered by tools**: fmt / clippy / `build.rs`
  gates / `scripts/ci/check-*.py` from P0 onward. Reviewers should focus only on what machines
  cannot check (responsibility classification, D3).

### What to Do When You Are Stuck

| Situation                                    | Correct action                                                                                                                                                          | Wrong action                                             |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| The criterion does not pass                  | Report honestly and fix the implementation; if you suspect the criterion is wrong, return to [07](architecture/07-equivalence-oracle.md) and propose a criterion change | Add a whitelist or exemption to the criterion            |
| A design premise does not hold               | Return to [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md), change the decision, and explain why                                               | Bypass the decision quietly                              |
| The task is larger than expected             | Break the level-3 task down further in [09](architecture/09-execution-wbs.md)                                                                                           | One oversized commit mixing multiple acceptance criteria |
| Documentation and code contradict each other | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` is reversed, D7)                                                     | Implement code according to incorrect documentation      |

### Pre-Commit Validation (Code Changes)

```bash
cargo fmt                                              # Formatting (required)
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # Must run before committing: lib + integration targets; see below
python scripts/ci/check-test-channel.py                # Keep the wording consistent across four places (this page / AGENTS.md / test-specification / pre-commit)
```

The target set for `run fast` is explicit: `--lib` (src/ unit tests) plus four integration targets:
`integration` / `formatter_integration` / `yx_runner` / `yx_multifile_runner`. The first stage of
`pre-commit` runs this command, and the second runs `cargo test --all --quiet` to block the commit.
Doctests (`cargo test --doc`) and corpus diffs are not covered by `run fast`; CI handles them. An
exclusive wave that does not fit the budget will list "skip N tests" at the end and provide a
fallback path; it must not skip tests silently.

Before pushing, also run the CI-only gates—they are not on the local checklist, so without them,
they effectively do not exist:

```bash
python scripts/ci/check-docs-truth.py
python scripts/ci/check-boundary.py
python scripts/ci/check-fanout.py
python scripts/ci/check-docs-refs.py
```

**The error-code reference page is a build-time artifact and is not committed to the repository**:
When the documentation site is built, `scripts/docs/gen-error-code-docs.py` regenerates it from the
code registry and `locales/*.json` (on both the zh and en sides), so it is no longer "regenerated
and committed locally." When changing a diagnostic code, change the authoritative source—the
`define_codes!` registry and its message text: every new code must also be added to both
`locales/zh.json` and `locales/en.json`; otherwise, the generated page will show "no entry in
locales" for that code. To preview the generated result, run `node docs/gen-error-docs.js`.

## 2. Changing Documentation

### Classification: Which Directory Does This Document Belong To?

Use the decision tree in [docs-rules.md](docs-rules.md) §1. The answer must be unique: tutorial /
guide / reference / explanation / dev / rfc. **If you cannot give a unique answer, this document is
serving two audiences—split it before doing any work.**

### Pre-Work Self-Check (Complete Every Item Before Changing Documentation)

- [ ] I used the docs-rules §1 decision tree to determine the directory, and the answer is unique
- [ ] I have grepped for similar content; the same content does not have ≥2 copies (if it does,
      merge them; do not add a third)
- [ ] I have checked docs/glossary.json and the existing documentation, reused existing terminology,
      and not invented a new name
- [ ] New document: the frontmatter `title` is set; it has been added to the sidebar (`config.js` +
      `i18n/en.json`; generateSidebar scans files in the directory, so no manual entry is needed)
- [ ] Moved or renamed document: all six synchronizations are complete (see "Six Synchronizations
      for Moving a Document" below)
- [ ] The language style passes docs-rules §2: active voice, declarative sentences, present tense,
      consistent terminology, and no marketing language

### Six Synchronizations for Moving a Document (Moving Means Changing the URL; None May Be Omitted)

1. Move it with `git mv` (preserving file history)
2. Site-wide links: grep for the old path and update each occurrence (the build's dead-link check
   will catch omissions, but it will not update them for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: move the corresponding English file in sync
5. Scripts and configurations that reference the path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, `README.md`
6. The cache keys in `docs/src/.i18n-cache.json` (the keys contain file paths)

### Is This a "Patch-Style Fix"? (Documentation D3—If Matched, Stop)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-Commit Validation (Documentation Changes)

```bash
cd docs
pnpm docs:build                                # Zero tolerance for dead links; the build must pass
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Execute documentation examples for real
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1s
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
python ../scripts/ci/check-docs-refs.py        # Reference paths must resolve (only paths in the repository count)
```

markdownlint cannot catch VitePress Vue compilation errors (angle-bracket placeholders are treated
as elements, producing `Element is missing end tag`), and in documentation-only commits it is
skipped with `no files to check`—**a real build is the only criterion**. All placeholders must be
placed in backticks.

If you changed anything under `rfc/`, add one more command:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## 3. Verification Measurement (Batch Trees)

Measurements must be performed on a **frozen revision**. Cargo fingerprints include source
paths—**switching trees means a full rebuild; switching revisions in the same tree is
incremental**—so use one tree per batch, always created inside the repository at
`target/yx-scratch/<batch>/tree` (already covered by `.gitignore`). Do not run `git worktree add` or
`git clone` directly, or create a directory outside the repository.

Flow: `init <batch>` creates the tree (the first full build takes about 5 minutes / about 3 GB) →
`use <batch> --rev <sha> --by <member> --task <id>` locks it and switches the revision →
`build <batch>` performs a standardized build (use the fixed `cargo build --bin yaoxiang-rs`; do not
invent your own command) → measure → `release <batch>` unlocks it → close out with `gc` and paste
the `status` output into the task output. For the parameters, see
`python scripts/dev/scratch.py --help`.

- **Only one holder is allowed at a time**; implementers do not work in this tree (they write code
  in the main repository working tree).
- Only when two people genuinely need to measure different revisions in parallel should you create a
  second tree, `<batch>-b`, and record the reason in the batch record (the cost is another full
  build).
- **Command forms for `trial`**: `trial <batch> --patch <p> -- <command...>` (recommended; tokens
  after `--` are passed through unchanged as the command, and the command may contain `-`/`--`
  options) and `trial <batch> <command...> --patch <p>` (usable when the command contains no option
  beginning with `-`) are equivalent. `--patch` must come before `--`; the command cannot be empty;
  mixing the two forms (commands on both sides of `--`) is rejected (exit 2).
- **You must run `build` after `trial` before measuring**: trial's mechanical restoration covers
  source files only, while the mutation test has already rebuilt the batch-tree binary—without
  rebuilding, all subsequent measurements come from the mutated artifact.
- Probes (`.yx`) also belong in `target/yx-scratch/<batch>/` (`target/` is ignored and does not
  enter corpus diffs); they are workspace artifacts, not documentation sources (see AGENTS Meta-Rule
  ①).

## References

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, review checklist (**the
  code-rule source of truth**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and wiki-style language (**the
  documentation-rule source of truth**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical incident
  table, external references, and rationale for rejected approaches (decision record, not a rule
  source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — level-3 task table, dependencies, and
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — authoritative responsibility-category definitions
  and target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — master outline and D1–D52 Decision
  Registry
