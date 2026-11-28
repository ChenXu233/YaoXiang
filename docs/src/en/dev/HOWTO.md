# Implementer's Handbook (HOWTO)

> **This page is the single entry point for all changes; it is not the canonical rule set.** The
> sole authority for the rules is:
>
> - Change code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise rule
>   set) and [RFC-039 decision registry](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - Change documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   Wikipedia-style language style)
>
> This page does only two things: **mandatory self-check before starting work** and **determining
> whether a change is a "patch-style fix."** If this page conflicts with the canonical rules, the
> canonical rules take precedence.

## Step 0: Determine What You Need to Change

| Your change                    | Section to read    | Canonical rules                    | Validation before commit  |
| ------------------------------ | ------------------ | ---------------------------------- | ------------------------- |
| Change code (`src/`, `tests/`) | Section 1          | [coding-rules.md](coding-rules.md) | See the end of Section 1  |
| Change documentation (`docs/`) | Section 2          | [docs-rules.md](docs-rules.md)     | See the end of Section 2  |
| Change both                    | Read both sections | Follow both rule sets              | Run both sets of commands |

Note: Code changes often require synchronized documentation changes—error code tables, CLI
arguments, and user-visible behavior are all asserted in the documentation. Passing Section 1's
acceptance criteria does not mean you are done. First answer: "Where is this behavior documented,
and does it need to be updated?"

## 1. Changing Code

### Which stage are you in?

The implementation master table is [09-execution-wbs.md](architecture/09-execution-wbs.md). **First
find your Level 3 task entry** (such as 4.2.7), and confirm: whether its prerequisites are complete,
which acceptance-criterion level it uses (C1–C6; the definitions are in
[07](architecture/07-equivalence-oracle.md)), and whether it is marked as an "exclusive commit."

### Pre-work self-check (complete every item before writing code)

- [ ] I read the original text of this task's Level 3 entry in
      [09](architecture/09-execution-wbs.md), not merely the task title
- [ ] I read the **actual context** of the code being changed (I opened the file rather than
      imagining it from the task description)
- [ ] I know the acceptance-criterion level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavioral equivalence / C5 AST+diagnostics+behavior / C6 deletion-only)
- [ ] I classified the **error class (A–F, per RFC-039 D59)**: A/B/C classes (the old behavior was
      **not successful**—a crash/stack overflow, silent failure, silent mismatch, or silently
      incorrect answer) may be fixed along the way during a refactor, but must go **red first, then
      green**, without changing the externally visible behavior of programs that previously
      succeeded; D/E/F classes (user-visible semantics/external contract tables/RFC-level design)
      require user approval first (see the
      [07 §Routing Criteria for Semantic Corrections](architecture/07-equivalence-oracle.md))
- [ ] I checked **D0**: Does this change touch any table (error codes/opcodes/types/stages)? Which
      module is the authoritative source? — If you cannot name the module, you have not investigated
- [ ] I checked **D1/D2**: Can newly added types/enums/constant tables be expressed using existing
      concepts + parameters? Are they semantically equivalent to existing concepts? (Do not paper
      over equivalent semantics with an import alias)
- [ ] I checked **D4**: Does the new code belong to an **existing responsibility category** in the
      target module? (See the directory responsibility table in
      [01-routing.md](architecture/01-routing.md) for the authoritative definitions)
- [ ] I applied the **trade-off test**: Are my rejection reasons (or those of the proposal's author)
      based solely on **correctness** and **readability**? Rejecting a proposal because it has a
      "large change surface," "requires a new mechanism," or "is expensive" is an invalid rejection,
      and review will send it back (part 2 of coding-rules); trade-offs requiring a user decision
      must provide all four items specified in the repository-root `AGENTS.md`
- [ ] Is my new mechanism of the "register-consume" type (one side registers data and the other
      consumes it)? If so, did the end-to-end tests that directly drive the producer and consumer
      ship in the same batch? A consumer test that manually constructs registered data is a silent
      path, and review will send it back (part 6 of coding-rules; precedent: D55 in the RFC-039
      decision registry)
- [ ] If this task requires writing or changing tests: I have read
      [test-specification.md](test-specification.md)—use the `test_<what>_<scenario>` prefix, add
      three-part AAA comments for tests longer than five lines, include custom assertion messages,
      match enum variants with `assert!(matches!(...))`, do not use `use super::*`, and do not use
      an inline `mod tests {}`
- [ ] If my change affects IR or corpus execution behavior (front end/ir_gen/std/corpus `.yx`): I
      plan to update the IR snapshots and corpus differential baseline **in the same commit**
      (commands in the common-commands section of AGENTS.md), and manually review both diffs—the
      parallel stream #385 once omitted a synchronized snapshot update and was caught by the gate
      (precedent: dabdfd96)
- [ ] If the task says "red first, then green" (such as 2.4.1 and 8.8.1), I have confirmed that the
      red criterion already exists and is currently failing
- [ ] I have not prepared to "delete failing tests" or "relax the acceptance criteria" just to
      pass—if that is impossible, the implementation is defective; report it truthfully rather than
      relax the criteria (D24/no C5′ exists)
- [ ] I asked myself: Where is this behavior documented, and does this change require a synchronized
      documentation update (change → Section 2)

### Is this a "patch-style fix"? (D3: stop as soon as one condition matches)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**The third condition is the reason this handbook exists.** "Changing ≥3 synonymous mappings" means
you are maintaining a parallel source of truth—the correct action is to eliminate the parallel
source, not add another one. Historical incidents: three sets of operator enums, five compiler entry
points, and a hand-written type synonym table (see the incident table in
[08](architecture/08-maintenance-mechanism.md)).

### Several hard rules that are easy to violate

- **Do not change code you have not inspected**: if you have not opened the file or grepped for
  references, you may not change it. Line numbers reflect the current repository; line numbers in
  documentation drift.
- **Do not leave unfinished work behind**: core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / "fix it later in a separate issue." If you find one, fix it
  or record it under the current phase's tasks—do not introduce new unfinished work (D52 precedent:
  the two genuine omissions in `.42` were all brought into P7; a recheck of the third,
  `exception_handlers`, found it was a false positive—a dead field, **already removed with P10 S6.1
  (6753bbe)**).
- **Verify line by line when verification is required**: when a task says "must verify line by line;
  equivalence cannot be assumed" (such as 4.2.7 and 6.1.3), include the verification result in the
  PR description.
- **Deleting tests to get a green build is a violation**: the DoD requires the test count at the end
  of each phase to be no lower than it was at the beginning.
- **Do not manually check mechanical work already covered by tools**: `fmt` / `clippy` / the
  `build.rs` gate / `scripts/ci/check-*.py` from P0 onward. Review should focus only on what
  machines cannot check (responsibility classification and D3).

### What to do when you are blocked

| Situation                                               | Correct action                                                                                                                                                                   | Incorrect action                                                 |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Acceptance criteria do not pass                         | Report the problem truthfully and fix the implementation; if you suspect the criteria are wrong, return to [07](architecture/07-equivalence-oracle.md) and propose changing them | Add a whitelist or exemption to the criteria                     |
| You find that a design premise does not hold            | Return to the [RFC-039 decision registry](../rfc/accepted/039-compiler-architecture.md), amend the decision, and explain why                                                     | Bypass the decision and proceed quietly                          |
| The task is larger than expected                        | Break the Level 3 task into finer pieces in [09](architecture/09-execution-wbs.md)                                                                                               | Use one oversized commit that mixes multiple acceptance criteria |
| You find a contradiction between documentation and code | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` was reversed, D7)                                                             | Write code according to incorrect documentation                  |

### Pre-commit validation (code changes)

```bash
cargo fmt                                              # Format (required)
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # Required before commit: lib + integration side, see below
python scripts/ci/check-test-channel.py                # One source of truth across all four locations (this page / AGENTS.md / test-specification / pre-commit)
python scripts/dev/test-gate-census.py --check         # The census table matches the current tree (pre-commit runs this automatically when .rs files / the census table change)
```

The target set for `run fast` is explicit: `--lib` (unit tests under `src/`) plus four
integration-side targets: `integration` / `formatter_integration` / `yx_runner` /
`yx_multifile_runner`. The first stage of `pre-commit` runs this command, and the second stage runs
`cargo test --all --quiet` to block the commit. Doctests (`cargo test --doc`) and corpus
differentials are not covered by `run fast`; CI handles them. An exclusive wave that does not fit
the budget lists "Skipped N tests" at the end and provides a fallback path; it never skips silently.

Before pushing, also run the CI-only gates—if they are not in the local checklist, they do not
exist:

```bash
python scripts/ci/check-docs-truth.py
python scripts/ci/check-boundary.py
python scripts/ci/check-fanout.py
python scripts/ci/check-docs-refs.py
```

**The error code reference page is a build-time artifact and is not checked in**: when the
documentation site is built, `scripts/docs/gen-error-code-docs.py` regenerates it from the code
registry and `locales/*.json` (on both the zh and en sides), so there is no longer a local
"regenerate and check in" step. When changing a diagnostic code, modify its authoritative source—the
`define_codes!` registry and message text: every new code must also have entries in
`locales/zh.json` and `locales/en.json`; otherwise, the generated page displays "no entry in
locales" for that code. To preview the generated output, run `node docs/gen-error-docs.js`.

## 2. Changing Documentation

### Classification: which directory this document belongs to

Use the decision tree in §1 of [docs-rules.md](docs-rules.md) to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this
document serves two audiences—split it before making any changes.**

### Pre-work self-check (complete every item before changing documentation)

- [ ] I used the §1 decision tree in docs-rules to determine the directory, and the answer is unique
- [ ] I grepped for similar content; the same content is not duplicated in two or more places (if it
      is, merge the copies; do not add a third)
- [ ] I checked `docs/glossary.json` and the existing documentation, reused existing terminology,
      and did not invent a new name
- [ ] New document: the frontmatter `title` is set; the sidebar entry has been added (`config.js` +
      `i18n/en.json`; generateSidebar scans files in the directory, so no manual link is needed)
- [ ] Moved or renamed document: all six synchronization steps are complete (see "Six
      synchronization steps for moving a document" below)
- [ ] The language style passes docs-rules §2: active voice, declarative statements, present tense,
      consistent terminology, and no marketing language

### Six synchronization steps for moving a document (a move changes the URL; none may be omitted)

1. Use `git mv` to move it (preserving file history)
2. Site-wide links: grep for the old path and update each occurrence (the build's dead-link check
   will catch omissions, but it will not update them for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. `en/` mirror: move the corresponding English file in sync
5. Scripts and configuration that reference the path: `scripts/`, `docs/scripts/`, `.github/`
   workflows, `AGENTS.md`, and `README.md`
6. The cache key in `docs/src/.i18n-cache.json` (the key contains the file path)

### Is this a "patch-style fix"? (documentation version of D3: stop as soon as one condition matches)

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
python ../scripts/ci/check-docs-examples.py    # Execute documentation examples for real
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1s
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
python ../scripts/ci/check-docs-refs.py        # Resolve reference paths (only committed paths count)
```

markdownlint cannot catch VitePress Vue compilation errors (an angle-bracket placeholder is treated
as an element, producing `Element is missing end tag`), and on a documentation-only commit it is
skipped with `no files to check`—**an actual build is the only criterion**. Always put placeholders
in backticks.

If you changed `rfc/`, add one more command:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## 3. Measurement (batch tree)

Measurements must be performed on a **frozen revision**. Cargo fingerprints include source
paths—**switching trees = a full rebuild; switching revisions within the same tree = an incremental
build**. Therefore, use one tree per batch, fixed at `target/yx-scratch/<batch>/tree` inside the
repository (already covered by `.gitignore`). Never run `git worktree add` / `git clone` or create a
directory outside the repository.

Workflow: `init <batch>` creates the tree (the first full build takes about 5 minutes / 3 GB) →
`use <batch> --rev <sha> --by <member> --task <id>` locks it and switches the revision →
`build <batch>` performs the standardized build (always use `cargo build --bin yaoxiang-rs`; do not
invent another command) → take measurements → `release <batch>` unlocks it → finish with `gc` and
paste the `status` output into the task output. See `python scripts/dev/scratch.py --help` for the
parameters.

- **Only one holder is allowed at a time**; implementers do not enter this tree (they write code in
  the main repository working tree).
- **When two people genuinely need to measure different revisions in parallel**, create a second
  tree, `<batch>-b`, and record the reason in the batch record (the cost is another full build).
- **`trial` command form**: `trial <batch> --patch <p> -- <command...>` (recommended; tokens after
  `--` are used verbatim as the command, and the command may contain `-`/`--` arguments) and
  `trial <batch> <command...> --patch <p>` (usable when the command has no arguments beginning with
  `-`) are equivalent. `--patch` must appear before `--`; the command must not be empty; mixing the
  two forms (commands on both sides of `--`) is rejected (exit 2).
- **You must run `build` after `trial` before measuring**: trial's mechanical restoration only
  covers source files, while the mutation test has already recompiled the batch-tree binary—without
  a rebuild, all subsequent measurements come from the mutated artifact.
- Probes (`.yx`) also go in `target/yx-scratch/<batch>/` (`target/` is ignored and is not included
  in corpus differentials); they are workspace artifacts, not documentation sources (see AGENTS
  meta-rule ①).

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, and the review
  checklist (**canonical code rules**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and Wikipedia-style language style
  (**canonical documentation rules**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident
  table, external references, and rejected-option rationale (a diagnostic record, not a rule source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level 3 task table, dependencies, and
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — Definitions of the C1–C6
  acceptance criteria
- [01-routing.md](architecture/01-routing.md) — Authoritative definitions of responsibility
  categories and the target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — Overall design and decision registry
  D1–D52
