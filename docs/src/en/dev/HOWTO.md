# Implementer’s Handbook (HOWTO)

> **This page is the single entry point for all changes; it is not the authoritative rule set.** The
> sole authority for the rules is:
>
> - Changing code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise rule
>   set) and [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - Changing documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory pre-work self-checks** and **determining whether a
> change is a “patch-style fix.”** If this page conflicts with the authoritative rule set, the rule
> set takes precedence.

## Step 0: Determine What You Are Changing

| Your change                      | Which section to read | Authoritative rule set             | Validation before commit  |
| -------------------------------- | --------------------- | ---------------------------------- | ------------------------- |
| Changing code (`src/`, `tests/`) | Section I             | [coding-rules.md](coding-rules.md) | See the end of Section I  |
| Changing documentation (`docs/`) | Section II            | [docs-rules.md](docs-rules.md)     | See the end of Section II |
| Changing both                    | Read both sections    | Follow both rule sets              | Run both sets of commands |

Note: Code changes often require corresponding documentation changes—error-code tables, CLI
arguments, and user-visible behavior are all asserted in the documentation. Passing Section I’s
acceptance checks does not mean the work is complete. First answer: “Where is this behavior
described, and does it need to change with the code?”

## I. Changing Code

### Which Stage You Are In

The implementation work breakdown is in [09-execution-wbs.md](architecture/09-execution-wbs.md).
**First locate the level-3 task entry** (for example, 4.2.7) and confirm:

- Whether its prerequisites are complete
- Which acceptance-criterion level it uses (C1–C6; the definitions are in
  [07](architecture/07-equivalence-oracle.md))
- Whether it is marked as a “dedicated commit”

### Pre-work self-check (complete every item before writing code)

- [ ] I read the full level-3 entry for this task in [09](architecture/09-execution-wbs.md), not
      just its title
- [ ] I read the **actual context** of the code being changed (by opening the file, not by inferring
      from the task description)
- [ ] I know this task’s acceptance-criterion level (C1 zero-diff / C2 diagnostic set / C3
      diagnostic codes / C4 behavioral equivalence / C5 AST + diagnostics + behavior / C6 pure
      deletion)
- [ ] I identified the **error category (A–F, per RFC-039 D59)**: Categories A/B/C (the old behavior
      was **unsuccessful**—a crash/stack overflow, silent failure, silent mismatch, or silently
      incorrect result) may be fixed incidentally during a refactor, but they must **fail before the
      fix and pass afterward** without changing the externally observable behavior of programs that
      previously succeeded. Categories D/E/F (user-visible semantics / external contract tables /
      RFC-level design) require a user decision first (the routing criteria are in
      [07 § Semantic-Correction Routing Criteria](architecture/07-equivalence-oracle.md))
- [ ] I ran **D0**: Does this change touch any table (error codes/opcodes/types/stages)? Which
      module is the authoritative source? If I cannot name the module, I have not investigated
- [ ] I ran **D1/D2**: Can the new type/enum/constant tables be expressed using existing concepts
      plus parameters? Do they have the same semantics as existing concepts? (Import aliases must
      not be used to paper over identical semantics)
- [ ] I ran **D4**: Does the new code belong to an existing responsibility category of the target
      module? (See the directory responsibility table in [01-routing.md](architecture/01-routing.md)
      for the authoritative definitions)
- [ ] I ran the **trade-off criterion**: Is my (or the proposal author’s) reason for rejecting the
      approach based only on **correctness** and **readability**? Rejecting an approach because it
      has “a large change surface / requires a new mechanism / is expensive” is invalid, and the
      review will be sent back (Part II of coding-rules). When a trade-off requires a user decision,
      provide all four items specified in the repository-root `AGENTS.md`
- [ ] Is my new mechanism of the “register–consume” type (one party registers data and another
      consumes it)? If so, are the end-to-end tests that directly exercise **both the producer and
      consumer** included in the same delivery? A consumer test that manually constructs registered
      data creates a silent path, and the review will be sent back (Part VI of coding-rules; see the
      precedent in RFC-039 Decision Register D55)
- [ ] If this task requires writing or changing tests: I read
      [test-specification.md](test-specification.md)—use the `test_<what>_<scenario>` prefix; use
      three-part AAA comments for tests longer than five lines; include custom assertion messages;
      match enum variants with `assert!(matches!(...))`; do not use `use super::*`; do not use an
      inline `mod tests {}`
- [ ] If my change affects IR or corpus execution behavior (frontend/ir_gen/std/corpus `.yx` files):
      I plan to update the IR snapshot and corpus-differential baseline **in the same commit** (see
      the common commands in AGENTS.md) and manually review both diffs—parallel stream #385 once
      missed a snapshot update and was caught by the gate (precedent: dabdfd96)
- [ ] If the task says to “fail before the fix and pass afterward” (for example, 2.4.1 or 8.8.1), I
      confirmed that the failing criterion already exists and currently fails
- [ ] I have not prepared to “delete failing tests” or “relax the criterion” merely to pass
      acceptance—**inability to satisfy it is an implementation defect; report it truthfully rather
      than weakening the criterion** (D24/C5′ does not exist)
- [ ] I asked myself where this behavior is documented and whether the documentation must change
      with the code (change → Section II)

### Is This a “Patch-Style Fix”? (D3: stop as soon as it applies)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Rule 3 is why this handbook exists.**

“Changing ≥3 synonymous mappings” means that you are maintaining a parallel source of truth. The
correct action is to eliminate the parallel source, not add another one. Historical incidents
include three sets of operator enums, five compiler entry points, and a hand-written table of type
synonyms (see the incident table in [08](architecture/08-maintenance-mechanism.md)).

### Hard Rules That Are Easy to Violate

- **Do not change code you have not read**: You may not modify code unless you have opened the file
  and searched for its references. Line numbers must reflect the current repository state because
  documentation line numbers drift.
- **Leave no deferred implementation work**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / “fix it later in a separate issue.” Fix anything you find or
  record it as a task for the current stage—do not introduce new deferred work (D52 precedent: all
  three instances of `.42` data loss have been incorporated into P7).
- **Verify verification items line by line**: When a task says that verification “must be performed
  line by line and equivalence must not be assumed” (for example, 4.2.7 or 6.1.3), record the
  verification result in the PR description.
- **Deleting tests to get a green build = a violation**: The DoD requires that the number of tests
  at the end of each stage must not be lower than it was at the beginning.
- **Do not manually check problems already covered by tools**: Do not manually check fmt / clippy /
  `build.rs` gates / `scripts/ci/check-*.py` from P0 onward. Reviews should focus only on what
  machines cannot check (responsibility classification and D3).

### If You Get Stuck

| Situation                        | Correct action                                                                                                                                                        | Incorrect action                                      |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| The criterion does not pass      | Report it truthfully and fix the implementation; if you suspect the criterion is wrong, return to [07](architecture/07-equivalence-oracle.md) and propose changing it | Add a whitelist or exemption to the criterion         |
| A design premise proves invalid  | Return to the [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md), amend the decision, and explain why                                          | Bypass the decision and proceed quietly               |
| The task is larger than expected | Break the level-3 task down further in [09](architecture/09-execution-wbs.md)                                                                                         | Mix multiple acceptance criteria into one huge commit |
| Documentation and code conflict  | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` was reversed, D7)                                                  | Write code according to incorrect documentation       |

### Pre-commit validation (code changes)

```bash
cargo fmt                                              # 格式化（必跑）
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # 提交前必跑：lib + 集成侧，见下
python scripts/ci/check-test-channel.py                # 四处口径同源（本页 / AGENTS.md / test-specification / pre-commit）
```

The target set for `run fast` is explicit: `--lib` (unit tests under `src/`) plus four
integration-side targets: `integration` / `formatter_integration` / `yx_runner` /
`yx_multifile_runner`. The first stage of `pre-commit` runs this command; the second runs
`cargo test --all --quiet` to block the commit. Doctests (`cargo test --doc`) and corpus
differentials are not covered by `run fast`; CI covers them. When a dedicated wave cannot fit within
the budget, it lists “skipped N tests” item by item at the end and provides a fallback path. It must
never skip tests silently.

Before pushing, also run the CI-only gates—without them, gates omitted from the local checklist are
effectively nonexistent:

```bash
python scripts/ci/check-docs-truth.py
python scripts/ci/check-boundary.py
python scripts/ci/check-fanout.py
python scripts/ci/check-docs-refs.py
python scripts/docs/gen-error-code-docs.py && git diff --quiet
```

Any change to a diagnostic code or its emission point (addition, deletion, or line-number shift)
requires rerunning the generator and confirming that `git diff --quiet`. Never edit generated output
by hand, and the commit message must explain the relevant category.

## II. Changing Documentation

### Classification: Which Directory Does This Page Belong To?

Use the decision tree in [docs-rules.md](docs-rules.md) §1. The answer must be unique (tutorial /
guide / reference / explanation / dev / rfc). **If you cannot determine a unique answer, this page
serves two audiences—split it before making any changes.**

### Pre-work self-check (complete every item before changing documentation)

- [ ] I used the docs-rules §1 decision tree to determine the directory, and the answer is unique
- [ ] I searched for similar content; no content has two or more copies (if it does, merge the
      copies rather than adding a third)
- [ ] I checked `docs/glossary.json` and the existing documentation, reused established terminology,
      and did not invent new names
- [ ] For new documentation: the frontmatter `title` is present; the page is linked from the sidebar
      (`config.js` + `i18n/en.json`; pages scanned by `generateSidebar` do not need manual linking)
- [ ] For moved or renamed documentation: all six synchronization points are complete (see “Six
      Synchronization Points for Moving Documentation” below)
- [ ] The language passes docs-rules §2: active voice, definite statements, present tense,
      consistent terminology, and no marketing language

### Six Synchronization Points for Moving Documentation (moving = changing the URL; none may be omitted)

1. Move the file with `git mv` (preserving file history)
2. Update site-wide links one by one: search for the old path and update every occurrence (the
   build’s broken-link check will catch omissions, but it will not update them for you)
3. Update the sidebar and navigation: `docs/src/.vitepress/config.js` and
   `docs/src/.vitepress/i18n/en.json`
4. Update the `en/` mirror: move the corresponding English file in the same way
5. Update scripts and configurations that reference the path: `scripts/`, `docs/scripts/`,
   `.github/` workflows, `AGENTS.md`, and `README.md`
6. Update the cache keys in `docs/src/.i18n-cache.json` (the keys contain file paths)

### Is This a “Patch-Style Fix”? (Documentation D3: stop as soon as it applies)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit validation (documentation changes)

```bash
cd docs
pnpm docs:build                                # 死链零容忍，构建必须绿
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # 文档示例真实执行
python ../scripts/ci/check-docs-orphan.py      # 孤儿页与重复 H1
python ../scripts/ci/check-docs-truth.py       # 版本注入与事实对账
python ../scripts/ci/check-docs-refs.py        # 引用路径可解析（入库路径才算）
```

Markdownlint cannot detect VitePress Vue compilation errors (angle-bracket placeholders are
interpreted as elements, producing `Element is missing end tag`). In documentation-only commits,
markdownlint reports `no files to check` and is skipped—**an actual build is the only criterion**.
Put every placeholder in backticks.

If you changed `rfc/`, add one more command:

```bash
python ../scripts/rfc/check_tracking.py        # RFC 状态一致性
```

## III. Verification Measurement (Batch Trees)

Measurements must be performed on a **frozen revision**. Cargo’s fingerprint includes source
paths—**changing trees causes a full rebuild, while switching revisions in the same tree is
incremental**. Therefore, use one tree per batch, always at `target/yx-scratch/<batch>/tree` inside
the repository (already covered by `.gitignore`). Do not use `git worktree add` / `git clone`
directly, and do not create directories outside the repository.

The workflow is: `init <batch>` creates the tree (the initial full build takes about 5 minutes / 3
GB) → `use <batch> --rev <sha> --by <member> --task <id>` locks it and switches revisions →
`build <batch>` performs a consistent build (always use `cargo build --bin yaoxiang-rs`; do not
invent another command) → take measurements → `release <batch>` unlocks it → close out with `gc` and
paste the `status` output into the task output. See `python scripts/dev/scratch.py --help` for the
parameters.

- **Only one holder is allowed at a time**; implementers do not work in this tree (they write code
  in the main repository working tree).
- Only when two people genuinely need to measure different revisions in parallel should you create a
  second tree, `<batch>-b`, and record the reason in the batch record (the cost is one additional
  full build).
- **You must run `build` after `trial` before measuring again**: the mechanical restoration
  performed by `trial` covers source files only, while the mutation test has already rebuilt the
  binary in the batch tree—without another build, every subsequent measurement comes from the
  mutated artifact.
- Put probes (`.yx` files) in `target/yx-scratch/<batch>/` as well (`target/` is ignored and is not
  included in corpus differentials); they are workspace artifacts, not documentation sources (see
  AGENTS Meta-Rule ①).

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, and the review
  checklist (**the authoritative code rules**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and wiki-style language (**the
  authoritative documentation rules**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident
  table, external references, and rejected-proposal rationales (decision records, not rule sources)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies, and
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — Authoritative responsibility-category definitions
  and target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — Overall plan and Decision Register
  D1–D52
