# Implementer's Manual (HOWTO)

> **This page is the single entry point for all changes; it is not the rule set itself.** The sole
> authority for the rules is:
>
> - To change code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rule set) and [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - To change documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style writing style)
>
> This page does only two things: **mandatory self-check before starting work** and **determine
> whether a change is a "patch-style fix."** If this page conflicts with the authoritative rules,
> the authoritative rules take precedence.

## Step 0: Determine What You Are Changing

| Your change                      | Section to read    | Authoritative rules                | Validation before committing |
| -------------------------------- | ------------------ | ---------------------------------- | ---------------------------- |
| Changing code (`src/`, `tests/`) | Section One        | [coding-rules.md](coding-rules.md) | See the end of Section One   |
| Changing documentation (`docs/`) | Section Two        | [docs-rules.md](docs-rules.md)     | See the end of Section Two   |
| Changing both                    | Read both sections | Follow both rule sets              | Run both sets of commands    |

Note: Changing code often requires documentation updates—error-code tables, CLI arguments, and
user-visible behavior are all asserted in the documentation. Passing Section One does not mean you
are done. First answer: "In which document is this behavior described, and does it need to change
too?"

## One. Changing Code

### Which stage are you in?

See the overall implementation plan: [09-execution-wbs.md](architecture/09-execution-wbs.md).
**First locate your level-three task entry** (for example, 4.2.7) and confirm its prerequisites,
acceptance level (the criteria are defined in [07](architecture/07-equivalence-oracle.md)), and
whether it is marked as an "exclusive commit."

### Pre-work self-check (complete every item before writing code)

- [ ] I have read the full level-three entry for this task in
      [09](architecture/09-execution-wbs.md), not just its title
- [ ] I have read the **actual context** of the code being changed (I opened the file rather than
      imagining it from the task description)
- [ ] I know the task's acceptance level (C1 zero-diff / C2 diagnostic set / C3 diagnostic codes /
      C4 behavioral equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have identified the **error category (A–F, per RFC-039 D59)**: A/B/C (the old behavior **did
      not succeed**—crash/stack overflow, silent failure, silent mismatch, or silently incorrect
      result) may be fixed alongside the refactor, but it must be **red first, then green**, without
      changing the externally observable behavior of programs that previously succeeded; D/E/F
      (user-visible semantics / external contract tables / RFC-level design) require a user decision
      first (for the criteria, see
      [07 §Semantic-correction routing criteria](architecture/07-equivalence-oracle.md))
- [ ] I have run **D0**: Does this change touch any table (error codes/opcodes/types/stages)? Which
      module is its source of truth? If you cannot name the module, you have not investigated
- [ ] I have run **D1/D2**: Can the new types/enums/constant tables be expressed using existing
      concepts + parameters? Are they semantically identical to existing concepts? (Do not paper
      over identical semantics with import aliases)
- [ ] I have run **D4**: Does the new code belong to an **existing responsibility category** of the
      target module? (For the authoritative responsibility-category definitions, see the directory
      responsibility table in [01-routing.md](architecture/01-routing.md))
- [ ] I have run the **trade-off criterion**: Is my (or the proposal author's) reason for rejection
      based solely on **correctness** and **readability**? Rejecting a proposal because it means "a
      large change footprint / introducing a new mechanism / high cost" is an invalid rejection, and
      review will send it back (Part Two of coding-rules); for trade-offs requiring a user decision,
      provide all four items required by the repository-root `AGENTS.md`
- [ ] Is my new mechanism of the "register–consume" kind (one side registers data and the other
      consumes it)? If so, have end-to-end tests that **directly exercise** both the producer and
      consumer been delivered in the same batch? A consumer test that manually constructs the
      registered data creates a silent channel, and review will send it back (Part Six of
      coding-rules; see the precedent in RFC-039 Decision Register D55)
- [ ] If this task requires writing or changing tests: I have read
      [test-specification.md](test-specification.md)—the `test_<what>_<scenario>` naming prefix, AAA
      comments in three parts for cases longer than five lines, assertions with custom messages,
      enum matching with `assert!(matches!(...))`, no `use super::*`, and no inline `mod tests {}`
- [ ] If my change affects IR or corpus runtime behavior (front end/ir_gen/std/corpus `.yx`): I plan
      to update the IR snapshot and corpus differential baseline **in the same commit** (see the
      common-commands section of AGENTS.md), and manually review both diffs—parallel stream #385
      once missed the synchronized snapshot and was blocked by the gate (precedent dabdfd96)
- [ ] If the task says "red first, then green" (for example, 2.4.1 or 8.8.1), I have confirmed that
      the red criterion already exists and is currently red
- [ ] I have not prepared to "delete failing tests" or "relax the criteria" to pass
      acceptance—**being unable to do so is an implementation defect; report it truthfully rather
      than using it as a reason to relax the criteria** (D24/no C5′)
- [ ] I have asked myself: In which document is this behavior described, and does this change
      require a corresponding documentation update? (Change → Section Two)

### Is this a "patch-style fix" (D3—stop as soon as one applies)?

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Rule 3 is the reason this manual exists.** "Changing ≥3 synonymous mappings" means maintaining a
parallel source of truth—the correct action is to eliminate the parallel source, not add another
one. Historical incidents include three sets of operator enums, five compiler entry points, and a
hand-written type-synonym table (see the incident table in
[08](architecture/08-maintenance-mechanism.md)).

### Several hard rules that are easy to violate

- **Do not change code you have not inspected**: If you have not opened the file or grepped for
  references, you may not modify it. Line numbers reflect the repository's current state; line
  numbers in documentation will drift.
- **Do not leave deferred implementations behind**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / "fix it in a separate issue later." If you find one, fix it
  or assign it to a task in the current phase—do not introduce new deferred work (D52 precedent: all
  three instances of data loss in `.42` have been incorporated into P7).
- **Verification items must be checked line by line**: When a task says "must be verified line by
  line; equivalence must not be assumed" (for example, 4.2.7 or 6.1.3), write the verification
  conclusion in the PR description.
- **Deleting tests to get a green result is a violation**: The DoD requires the number of tests at
  the end of each phase to be no lower than at the beginning.
- **Do not manually check mechanical problems already covered by tools**: fmt / clippy / `build.rs`
  gates / `scripts/ci/check-*.py` from P0 onward. Review should focus only on what machines cannot
  check (responsibility determination and D3).

### What to do when you are blocked

| Situation                                                  | Correct action                                                                                                                                                                    | Incorrect action                                             |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| The acceptance criteria do not pass                        | Report the failure honestly and fix the implementation; if you suspect the criteria are wrong, return to [07](architecture/07-equivalence-oracle.md) and propose a change to them | Add a whitelist or exemption to the criteria                 |
| You discover that a design premise does not hold           | Return to the [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md), amend the decision, and explain why                                                      | Bypass the decision and proceed quietly                      |
| The task is larger than expected                           | Break the level-three task down further in [09](architecture/09-execution-wbs.md)                                                                                                 | Use one oversized commit to mix multiple acceptance criteria |
| You find a conflict between the documentation and the code | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` is reversed, D7)                                                               | Write code according to incorrect documentation              |

### Pre-commit validation (code changes)

```bash
cargo fmt                                              # Format (required)
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # Required before committing: lib + integration targets; see below
python scripts/ci/check-test-channel.py                # Keep one source of truth across four places (this page / AGENTS.md / test-specification / pre-commit)
python scripts/dev/test-gate-census.py --check         # The census table matches the current tree (pre-commit runs automatically when .rs files / the census table change)
```

The target set for `run fast` is explicit: `--lib` (src/ unit tests) plus four integration targets:
`integration` / `formatter_integration` / `yx_runner` / `yx_multifile_runner`. The first stage of
`pre-commit` runs this command, and the second runs `cargo test --all --quiet` to block commits.
Doctests (`cargo test --doc`) and corpus differentials are not covered by `run fast`; CI handles
them. An exclusive wave that does not fit the budget will list each "skipped N tests" at the end and
provide a fallback path; it never skips tests silently.

Before pushing, also run the CI-only gates. If they are not on the local list, they do not exist:

```bash
python scripts/ci/check-docs-truth.py
python scripts/ci/check-boundary.py
python scripts/ci/check-fanout.py
python scripts/ci/check-docs-refs.py
```

**The error-code reference page is a build-time artifact and is not checked in**: When the
documentation site is built, `scripts/docs/gen-error-code-docs.py` regenerates it from the code
registry and `locales/*.json` (for both zh and en), so it is no longer "regenerated and checked in"
locally. When changing a diagnostic code, change its source of truth—the `define_codes!` registry
and the message text: every new code must also have entries in `locales/zh.json` and
`locales/en.json`; otherwise, the generated page will show "no entry in locales" for that code. To
preview the generated output, run `node docs/gen-error-docs.js`.

## Two. Changing Documentation

### Classification: Which directory does this document belong to?

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer. The answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this
document is serving two audiences—split it before starting work.**

### Pre-work self-check (complete every item before changing documentation)

- [ ] I used the docs-rules §1 decision tree to determine the directory, and the answer is unique
- [ ] I grepped for similar content; the same content does not have ≥2 copies (if it does,
      consolidate them and do not add a third)
- [ ] I checked docs/glossary.json and the existing documentation, followed existing terminology,
      and did not invent new names
- [ ] New document: the frontmatter `title` has been written, and the sidebar entry has been added
      (`config.js` + `i18n/en.json`; generateSidebar scans files in the directory, so no manual
      entry is needed)
- [ ] Moving/renaming a document: all six synchronization tasks are complete (see "Six
      synchronization tasks for moving a document" below)
- [ ] The language style passes docs-rules §2: active voice, definite statements, present tense,
      consistent terminology, and no marketing language

### Six synchronization tasks for moving a document (moving = changing the URL; none may be omitted)

1. Use `git mv` to move it (preserving file history)
2. Site-wide links: grep for the old path and update each occurrence (the build's dead-link check
   will catch mistakes, but will not do this for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: move the corresponding English file in sync
5. Scripts and configurations that reference the path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, `README.md`
6. The cache key in `docs/src/.i18n-cache.json` (the key contains the file path)

### Is this a "patch-style fix" (documentation version of D3—stop as soon as one applies)?

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit validation (documentation changes)

```bash
cd docs
pnpm docs:build                                # Zero tolerance for dead links; the build must pass
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Actually execute documentation examples
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1s
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
python ../scripts/ci/check-docs-refs.py        # Reference paths resolve (only checked-in paths count)
```

markdownlint cannot catch VitePress Vue compilation errors (angle-bracket placeholders are treated
as elements, resulting in `Element is missing end tag`), and in documentation-only changes it is
skipped with `no files to check`—**the real build is the only criterion**. All placeholders must be
enclosed in backticks.

If you changed anything under `rfc/`, also run:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## Three. Validation Measurements (Batch Trees)

Measurements must be performed on a **frozen revision**. Cargo fingerprints include source
paths—**switching trees means a full rebuild, while switching revisions in the same tree is
incremental**—so each batch gets one tree, fixed inside the repository at
`target/yx-scratch/<batch>/tree` (already covered by `.gitignore`). Do not run `git worktree add` /
`git clone` directly or create a directory outside the repository.

Workflow: `init <batch>` creates the tree (the initial full build takes about 5 minutes / about 3
GB) → `use <batch> --rev <sha> --by <member> --task <id>` locks it and switches revisions →
`build <batch>` performs the standardized build (use the fixed `cargo build --bin yaoxiang-rs`; do
not invent commands) → measure → `release <batch>` unlocks it → finalize with `gc` and paste the
`status` output into the task output. For parameters, see `python scripts/dev/scratch.py --help`.

- **Only one holder is allowed at a time**; implementers do not work in this tree (they write code
  in the main repository working tree).
- Only when two people genuinely need to measure different revisions in parallel should a second
  `<batch>-b` tree be created, and the reason must be recorded in the batch record (the cost is
  another full build).
- **The `trial` command forms**: `trial <batch> --patch <p> -- <command...>` (recommended; tokens
  after `--` are used verbatim as the command, which may contain `-`/`--` arguments) and
  `trial <batch> <command...> --patch <p>` (usable when the command does not begin with a `-`
  argument) are equivalent. `--patch` must appear before `--`; the command must not be empty; mixing
  the two forms (placing commands on both sides of `--`) is rejected (exit 2).
- **A `build` must follow `trial` before measurement**: Trial's mechanical restoration covers only
  source files, and the mutation test has already recompiled the batch-tree binary—without a
  rebuild, all subsequent measurements come from the mutated artifact.
- Probes (`.yx`) also go in `target/yx-scratch/<batch>/` (`target/` is ignored and is not included
  in corpus differentials); a probe is a workspace artifact, not a documentation source (see AGENTS
  meta-rule ①).

## References

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, and the review
  checklist (**the authoritative code rules**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and wiki-style writing style (**the
  authoritative documentation rules**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical incident
  table, external references, and rationale for rejected proposals (decision log, not a source of
  rules)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — level-three task table, dependencies,
  and parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — definitions of the C1–C6
  criteria
- [01-routing.md](architecture/01-routing.md) — authoritative responsibility-category definitions
  and target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — overview and Decision Register D1–D52
