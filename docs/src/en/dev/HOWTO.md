# Implementer's Handbook (HOWTO)

> **This page is the single entry point for all changes; it is not the authoritative rule set.** The
> sole authority for the rules is:
>
> - Changing code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise rule
>   set) and [RFC-039 decision registry](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - Changing documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory pre-work self-checks** and **determining whether a
> change is a "patch-style fix."** If this page conflicts with the authoritative rules, the
> authoritative rules take precedence.

## Step 0: Determine What You Are Changing

| Your change                      | Which section to read | Authoritative rules                | Validation before commit   |
| -------------------------------- | --------------------- | ---------------------------------- | -------------------------- |
| Changing code (`src/`, `tests/`) | Section I             | [coding-rules.md](coding-rules.md) | See the end of Section I   |
| Changing documentation (`docs/`) | Section II            | [docs-rules.md](docs-rules.md)     | See the end of Section II  |
| Changing both                    | Read both sections    | Follow both rule sets              | Run both validation suites |

Note: Code changes often require corresponding documentation changes: error-code tables, CLI
arguments, and user-visible behavior are all asserted in the documentation. Passing Section I's
acceptance checks does not mean the work is complete. First answer: “Where is this behavior
described, and does it need to change as well?”

## I. Changing Code

### Which Stage You Are In

The master execution table is [09-execution-wbs.md](architecture/09-execution-wbs.md). **First
locate the level-three task entry for your work** (such as 4.2.7), then confirm:

- Whether its prerequisites are complete
- Which acceptance criterion tier it uses (C1–C6; the criteria are defined in
  [07](architecture/07-equivalence-oracle.md))
- Whether it is marked as an "exclusive commit"

### Pre-Work Self-Check (Complete Every Item Before Writing Code)

- [ ] I have read the full level-three entry for this task in
      [09](architecture/09-execution-wbs.md), not just the task title
- [ ] I have read the **actual context** of the code being changed (I opened the file; I did not
      imagine it from the task description)
- [ ] I know the acceptance criterion tier for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic codes / C4 behavioral equivalence / C5 AST + diagnostics + behavior / C6 deletion
      only)
- [ ] I have classified the **error type (A–F, per RFC-039 D59)**: A/B/C (the old behavior **did not
      succeed**—crash/stack overflow, silent failure, silent mismatch, or silently returning the
      wrong answer) may be fixed as part of the refactoring, but the fix must **go red before
      green** and must not change the externally observable behavior of programs that previously
      succeeded; D/E/F (user-visible semantics/external contract tables/RFC-level design) require a
      user decision first (see the routing criteria in
      [07 §Semantic-correction routing criteria](architecture/07-equivalence-oracle.md))
- [ ] I ran **D0**: Does this change touch any table (error codes/opcodes/types/phases)? Which
      module is the authoritative source? — If you cannot name the module, you have not investigated
- [ ] I ran **D1/D2**: Can the newly added type/enum/constant tables be expressed using existing
      concepts plus parameters? Are they semantically identical to existing concepts? (If the
      semantics are identical, do not bridge the difference with an import alias)
- [ ] I ran **D4**: Does the new code belong to a **responsibility category that already exists** in
      the target module? (See the directory responsibility table in
      [01-routing.md](architecture/01-routing.md) for the authoritative definitions)
- [ ] I ran the **trade-off test**: Is my (or the proposal author's) reason for rejecting a solution
      based solely on **correctness** and **readability**? Rejecting a proposal because it has "a
      large change surface / requires a new mechanism / is expensive" is an invalid rejection, and
      review will send it back (Part 2 of coding-rules); for trade-offs that require a user
      decision, provide all four elements specified by the repository-root `AGENTS.md`
- [ ] Is my new mechanism of the "register–consume" type (one side registers data and the other
      consumes it)? If so, were the end-to-end tests that **directly drive** both the producer and
      consumer delivered in the same batch? A consumer test that manually constructs the
      registration data creates a silent channel, and review will send it back (Part 6 of
      coding-rules; see the precedent in RFC-039 decision registry D55)
- [ ] If this task requires writing or changing tests: I have read
      [test-specification.md](test-specification.md)—use the `test_<what>_<scenario>` prefix, AAA
      comments in three parts for blocks longer than 5 lines, include custom messages in assertions,
      match enums with `assert!(matches!(...))`, do not use `use super::*`, and do not use an inline
      `mod tests {}`
- [ ] If my change affects IR or corpus execution behavior (frontend/ir_gen/std/corpus `.yx`): I
      plan to update the IR snapshot and corpus-diff baseline **in the same commit** (see the common
      commands in AGENTS.md), and I will manually review both diffs—the parallel stream #385 once
      failed to synchronize the snapshot and was blocked by the gate (precedent: dabdfd96)
- [ ] If the task says to "go red before green" (such as 2.4.1 and 8.8.1), I have confirmed that the
      red criterion already exists and is currently failing
- [ ] I have not prepared to "delete failing tests" or "relax the criteria" to pass
      acceptance—**inability to satisfy the criteria is an implementation defect and must be
      reported honestly, not a reason to relax them** (D24/no C5′)
- [ ] I have asked myself where this behavior is documented and whether the documentation must
      change with this modification (change → Section II)

### Is This a "Patch-Style Fix" (D3; Stop if It Matches)?

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Rule 3 is the reason this handbook exists.**

Changing ≥3 synonymous mappings means maintaining a parallel source of truth—the correct action is
to eliminate the parallelism, not add another location. Historical incidents include three sets of
operator enums, five compiler entry points, and a hand-written table of type synonyms (see the
incident table in [08](architecture/08-maintenance-mechanism.md)).

### Several Hard Rules That Are Easy to Violate

- **Do not change code you have not inspected**: If you have not opened the file or grepped for
  references, you may not modify it. Use line numbers from the current repository state;
  documentation line numbers drift.
- **Do not leave deferred implementation work**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / "fix it in a separate issue later". If you find one, fix it
  or record it as a task in the current phase; do not introduce new deferred work (D52 precedent:
  all three `.42` data-loss sites have been brought under P7).
- **Verification items must be verified line by line**: When a task is marked "must be verified line
  by line; equivalence must not be assumed" (such as 4.2.7 and 6.1.3), record the verification
  conclusion in the PR description.
- **Deleting tests to obtain a green build = violation**: The DoD requires that the number of tests
  at the end of each phase must not be lower than it was at the beginning.
- **Do not manually check mechanical issues already covered by tools**: fmt / clippy / `build.rs`
  gates / `scripts/ci/check-*.py` from P0 onward. Review should focus only on what machines cannot
  check (responsibility classification and D3).

### If You Get Stuck

| Situation                              | Correct action                                                                                                                                                | Incorrect action                                           |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| The criterion does not pass            | Report honestly and fix the implementation; if you suspect the criterion is wrong, return to [07](architecture/07-equivalence-oracle.md) and propose a change | Add a whitelist or exemption to the criterion              |
| A design premise proves invalid        | Return to [RFC-039 decision registry](../rfc/accepted/039-compiler-architecture.md), change the decision, and explain why                                     | Bypass the decision and proceed quietly                    |
| The task is larger than expected       | Break the level-three task down further in [09](architecture/09-execution-wbs.md)                                                                             | Mix multiple acceptance criteria into one oversized commit |
| The documentation contradicts the code | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` was reversed, D7)                                          | Write code according to incorrect documentation            |

### Pre-Commit Validation (Code Changes)

```bash
cargo fmt                                              # Format (required)
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # Required before commit: lib + integration side, see below
python scripts/ci/check-test-channel.py                # Consistent across four locations (this page / AGENTS.md / test-specification / pre-commit)
```

The target set for `run fast` is explicit: `--lib` (unit tests under src/) plus four
integration-side targets: `integration` / `formatter_integration` / `yx_runner` /
`yx_multifile_runner`. The first stage of `pre-commit` runs this command; the second stage runs
`cargo test --all --quiet` to block the commit. Doctests (`cargo test --doc`) and corpus diffing are
outside the scope of `run fast` and are covered by CI. An exclusive wave that does not fit the
budget lists “skipped N tests” individually at the end and provides a degraded path; it never skips
silently.

Before pushing, also run the CI-only gates. If a gate is not on the local checklist, it does not
exist:

```bash
python scripts/ci/check-docs-truth.py
python scripts/ci/check-boundary.py
python scripts/ci/check-fanout.py
python scripts/ci/check-docs-refs.py
python scripts/docs/gen-error-code-docs.py && git diff --quiet
```

Any change to a diagnostic code or its emission site—including additions, removals, or line-number
shifts—requires rerunning the generator and confirming `git diff --quiet`. Never edit generated
artifacts by hand, and the commit message must classify and explain the change.

## II. Changing Documentation

### Classification: Which Directory This Document Belongs To

Use the decision tree in [docs-rules.md](docs-rules.md) §1. The answer must be unique (tutorial /
guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this document
serves two audiences—split it before making any changes.**

### Pre-Work Self-Check (Complete Every Item Before Changing Documentation)

- [ ] I used the docs-rules §1 decision tree to determine the directory, and the answer is unique
- [ ] I grepped for similar content; the same content does not exist in ≥2 locations (if it does,
      consolidate it rather than adding a third copy)
- [ ] I checked docs/glossary.json and the existing documentation, followed established terminology,
      and did not invent a new name
- [ ] New document: the frontmatter `title` is set; the sidebar entry is connected (`config.js` +
      `i18n/en.json`; generateSidebar scans files in the directory, so no manual entry is needed)
- [ ] Moved/renamed document: all six synchronizations are complete (see “Six Synchronizations for
      Moving a Document” below)
- [ ] The language style passes docs-rules §2: active voice, declarative sentences, present tense,
      consistent terminology, and no marketing language

### Six Synchronizations for Moving a Document (Moving = Changing the URL; None May Be Omitted)

1. Use `git mv` to move the file (preserving file history)
2. Site-wide links: grep for the old path and update each occurrence (the build's dead-link check
   provides a backstop, but does not do this for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: move the corresponding English file in sync
5. Scripts and configurations that reference the path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, and `README.md`
6. The cache key in `docs/src/.i18n-cache.json` (the key contains the file path)

### Is This a "Patch-Style Fix" (Documentation D3; Stop if It Matches)?

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-Commit Validation (Documentation Changes)

```bash
cd docs
pnpm docs:build                                # No tolerance for dead links; the build must pass
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Execute documentation examples for real
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
python ../scripts/ci/check-docs-refs.py        # Resolve referenced paths (only repository paths count)
```

markdownlint cannot catch VitePress Vue compilation errors (angle-bracket placeholders are treated
as elements and produce `Element is missing end tag`), and in documentation-only commits it is
skipped with `no files to check`—**a real build is the only criterion**. Put every placeholder in
backticks.

If you changed `rfc/`, add one more command:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## III. Measurement (Batch Tree)

Measurements must be performed on a **frozen revision**. Cargo's fingerprint includes source
paths—**switching trees means a full rebuild; switching revisions within the same tree is
incremental**—so use one tree per batch, fixed inside the repository at
`target/yx-scratch/<batch>/tree` (already covered by `.gitignore`). Never use `git worktree add` or
`git clone` directly, and never create a directory outside the repository.

Process: `init <batch>` creates the tree (the initial full build takes about 5 minutes / about 3 GB)
→ `use <batch> --rev <sha> --by <member> --task <id>` locks the tree and switches revisions →
`build <batch>` performs the unified build (the fixed command is `cargo build --bin yaoxiang-rs`; do
not invent another command) → measure → `release <batch>` unlocks it → close out with `gc` and paste
the `status` output into the task output. See `python scripts/dev/scratch.py --help` for the
parameters.

- **Only one holder is allowed at a time**; implementers do not enter this tree (they write code in
  the main repository working tree).
- If two people genuinely need to measure different revisions in parallel, create a second
  `<batch>-b` tree and document the reason in the batch record (the cost is another full build).
- **Command forms for `trial`**: `trial <batch> --patch <p> -- <command...>` (recommended; the
  tokens after `--` are used verbatim as the command, and the command may contain `-`/`--`
  arguments) and `trial <batch> <command...> --patch <p>` (equivalent when the command has no
  argument beginning with `-`) are equivalent. `--patch` must appear before `--`; the command must
  not be empty; mixing the two forms (commands on both sides of `--`) is rejected (exit 2).
- **You must run `build` after `trial` before measuring**: trial's mechanical restoration covers
  only source files, and the mutant tests have already recompiled the batch-tree binaries; without
  rebuilding, every subsequent measurement comes from the mutated artifacts.
- Probes (`.yx`) also belong in `target/yx-scratch/<batch>/` (`target/` is ignored and is not part
  of corpus diffing); they are workspace artifacts, not documentation sources (see AGENTS meta-rule
  ①).

## References

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, and the review
  checklist (**authoritative code rules**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and wiki-style language
  (**authoritative documentation rules**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical incident
  table, external references, and rejected-proposal analysis (diagnostic records, not rule sources)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — level-three task table, dependencies,
  and parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 acceptance criteria
  definitions
- [01-routing.md](architecture/01-routing.md) — authoritative responsibility-category definitions
  and target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — overview and D1–D52 decision registry
