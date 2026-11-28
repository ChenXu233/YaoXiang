# Implementer's Handbook (HOWTO)

> **This page is the single entry point for all changes; it is not the canonical rules.** The sole
> authority for the rules is:
>
> - Changing code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the exact rules)
>   and [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - Changing documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language style)
>
> This page does only two things: **mandatory pre-work self-check** and **determining whether a
> change is a “patch-style fix.”** If this page conflicts with the canonical rules, the canonical
> rules take precedence.

## Step 0: Determine What You Are Changing

| Your change                      | Section to read | Canonical rules                    | Validation before committing |
| -------------------------------- | --------------- | ---------------------------------- | ---------------------------- |
| Changing code (`src/`, `tests/`) | Section I       | [coding-rules.md](coding-rules.md) | See the end of Section I     |
| Changing documentation (`docs/`) | Section II      | [docs-rules.md](docs-rules.md)     | See the end of Section II    |
| Changing both                    | Read both       | Follow both sets of rules          | Run both sets of commands    |

Note: Code changes often require corresponding documentation changes—error-code tables, CLI flags,
and user-visible behavior are all asserted in the documentation. Passing validation in Section I
does not mean you are finished. First answer: “Where is this behavior documented, and does it need
to be updated accordingly?”

## I. Changing Code

### Which Phase Are You In?

The execution work breakdown is [09-execution-wbs.md](architecture/09-execution-wbs.md). **First
locate your level-3 task item** (such as 4.2.7) and confirm:

- Whether its prerequisites are complete
- Which acceptance-criterion level it uses (C1–C6; definitions are in
  [07](architecture/07-equivalence-oracle.md))
- Whether it is marked as an “exclusive commit”

### Pre-Work Self-Check (Complete Every Item Before Writing Code)

- [ ] I read the full text of this task's level-3 item in [09](architecture/09-execution-wbs.md),
      not just its title
- [ ] I read the **actual context** of the code being changed by opening the file, rather than
      imagining it from the task description
- [ ] I know this task's acceptance-criterion level (C1 zero-diff / C2 diagnostic set / C3
      diagnostic codes / C4 behavioral equivalence / C5 AST + diagnostics + behavior / C6 deletion
      only)
- [ ] I identified the **error category (A–F, per RFC-039 D59)**: A/B/C categories (legacy behavior
      **did not succeed**—crash/stack overflow, silent failure, silent mismatch, or silently
      returning the wrong answer) may be fixed incidentally during refactoring, but they must be
      **red before green** and must not change the externally observable behavior of programs that
      previously succeeded; D/E/F categories (user-visible semantics/external contract
      table/RFC-level design) require a user decision first (see the criteria in
      [07 §Routing Criteria for Semantic Corrections](architecture/07-equivalence-oracle.md))
- [ ] I ran **D0**: Does this change touch any table (error codes/opcodes/types/stages)? Which
      module is the authoritative source? — If you cannot name the module, you have not investigated
- [ ] I ran **D1/D2**: Can the new type/enum/constant table be expressed using existing concepts +
      parameters? Does it have the same semantics as an existing concept? (Do not paper over
      semantically equivalent concepts with import aliases)
- [ ] I ran **D4**: Does the new code belong to an **existing responsibility category** in the
      target module? (See the directory responsibility table in
      [01-routing.md](architecture/01-routing.md) for authoritative definitions)
- [ ] I ran the **trade-off criterion**: Is my rejection rationale—or that of the proposal's
      author—based solely on **correctness** and **readability**? Rejecting a proposal because it
      has a “large change surface / introduces a new mechanism / is expensive” is an invalid
      rejection, and review will send it back (Part 2 of coding-rules); for trade-offs requiring a
      user decision, provide all four items specified by the repository-root `AGENTS.md`
- [ ] Is my new mechanism of the “register-consume” type (one side registers data and the other
      consumes it)? If so, were the producer-side and consumer-side **directly driven** end-to-end
      tests delivered in the same batch? A consumer-side test that manually constructs registration
      data is a silent path, and review will send it back (Part 6 of coding-rules; precedent:
      RFC-039 Decision Register D55)
- [ ] If this task involves writing or changing tests: I read
      [test-specification.md](test-specification.md)—the `test_<what>_<scenario>` naming prefix, AAA
      three-section comments for sequences longer than five lines, assertions with custom messages,
      enum matching with `assert!(matches!(...))`, no `use super::*`, and no inline `mod tests {}`
- [ ] If my change affects IR or corpus execution behavior (frontend/ir_gen/std/corpus `.yx`): I
      plan to update the IR snapshot and corpus differential baseline **in the same commit** (see
      the common-commands section of AGENTS.md), and manually review both diffs—the parallel stream
      #385 once missed a snapshot update and was blocked by the gate (precedent: dabdfd96)
- [ ] If the task says “red before green” (such as 2.4.1 or 8.8.1), I confirmed that the red
      criterion already exists and is currently failing
- [ ] I have not prepared to “delete failing tests” or “relax the criteria” to pass
      acceptance—**inability to do so is an implementation defect; report it truthfully rather than
      relaxing the criteria** (D24/no C5′)
- [ ] I asked myself where this behavior is documented and whether the documentation must be updated
      with this change (change → Section II)

### Is This a “Patch-Style Fix”? (D3—Stop at the First Match)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Rule 3 is the reason this handbook exists.**

Changing ≥3 synonymous mappings means maintaining a parallel source of truth—the correct action is
to eliminate the parallelism, not add another instance. Historical incidents include three sets of
operator enums, five compiler entry points, and a hand-written type-synonym table (see the incident
table in [08](architecture/08-maintenance-mechanism.md)).

### Several Hard Rules That Are Easy to Violate

- **Do not change code you have not read**: You may not modify anything unless you have opened the
  file and grepped its reference sites. Line numbers reflect the repository's current state;
  documentation line numbers drift.
- **Do not leave pending implementation artifacts**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / “fix it in a separate issue later.” If you find one, fix it
  or record it in the current phase's task—do not introduce new leftovers (D52 precedent: the two
  genuine omissions in `.42` were all incorporated into P7; the third, `exception_handlers`, was
  rechecked and found to be a false positive—a dead field whose deletion was assigned to P10 S6).
- **Items requiring verification must be checked line by line**: When a task is marked “must be
  verified line by line; equivalence must not be assumed” (such as 4.2.7 or 6.1.3), include your
  verification conclusions in the PR description.
- **Deleting tests to get a green build is a violation**: The DoD requires the test count at the end
  of each phase to be no lower than it was at the beginning.
- **Do not manually check parts already covered by tooling**: fmt / clippy / `build.rs` gates /
  `scripts/ci/check-*.py` from P0 onward. Reviews should focus only on what machines cannot check
  (responsibility classification and D3).

### What If You Get Stuck?

| Situation                          | Correct action                                                                                                                                                     | Incorrect action                                                 |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| The criterion does not pass        | Report truthfully and fix the implementation; if you believe the criterion is wrong, return to [07](architecture/07-equivalence-oracle.md) and propose changing it | Add a whitelist or exemption to the criterion                    |
| A design premise proves invalid    | Return to the [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md), amend the decision, and explain why                                       | Quietly work around the decision                                 |
| The task is larger than expected   | Split the level-3 task further in [09](architecture/09-execution-wbs.md)                                                                                           | Use one oversized commit that mixes multiple acceptance criteria |
| Documentation contradicts the code | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` is reversed, D7)                                                | Write code according to the incorrect documentation              |

### Validation Before Committing (Code Changes)

```bash
cargo fmt                                              # Formatting (required)
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # Required before commit: lib + integration side; see below
python scripts/ci/check-test-channel.py                # Keep the wording consistent across four locations (this page / AGENTS.md / test-specification / pre-commit)
python scripts/dev/test-gate-census.py --check         # The census table matches the current tree (pre-commit runs this automatically when .rs / census table files change)
```

The target set for `run fast` is explicit: `--lib` (unit tests under src/) plus four
integration-side targets: `integration` / `formatter_integration` / `yx_runner` /
`yx_multifile_runner`. The first stage of `pre-commit` runs this exact command; the second stage
runs `cargo test --all --quiet` to block the commit. Doctests (`cargo test --doc`) and corpus
differentials are not covered by `run fast`; CI handles them. Exclusive waves that exceed the budget
list each skipped test as “Skipping N tests” at the end and provide a fallback path; nothing is
skipped silently.

Before pushing, also run the CI-only gates. They are not in the local checklist, so they do not
exist until you run them:

```bash
python scripts/ci/check-docs-truth.py
python scripts/ci/check-boundary.py
python scripts/ci/check-fanout.py
python scripts/ci/check-docs-refs.py
```

**The error-code reference page is a build-time artifact and is not committed**: When the
documentation site is built, `scripts/docs/gen-error-code-docs.py` regenerates it from the code
registry and `locales/*.json` for both zh and en. Therefore, there is no longer a local “regenerate
and commit” step. When changing a diagnostic code, change its authoritative sources—the
`define_codes!` registry and its text. Every new code must also have entries in both
`locales/zh.json` and `locales/en.json`; otherwise, the generated page displays “no entry in
locales.” To preview the generated result, run `node docs/gen-error-docs.js`.

## II. Changing Documentation

### Classification: Which Directory Does This Document Belong To?

Use the decision tree in §1 of [docs-rules.md](docs-rules.md). The answer must be unique (tutorial /
guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this document
serves two audiences—split it before making any changes.**

### Pre-Work Self-Check (Complete Every Item Before Changing Documentation)

- [ ] I used the §1 decision tree in docs-rules to determine the directory, and the answer is unique
- [ ] I grepped for similar content; the same content does not exist in ≥2 places (if it does,
      consolidate it and do not add a third copy)
- [ ] I checked `docs/glossary.json` and existing documentation, reused existing terminology, and
      did not coin a new name
- [ ] For a new document: the frontmatter `title` is set; the sidebar entry is present
      (`config.js` + `i18n/en.json`; generateSidebar scans files in the directory, so no manual
      entry is needed)
- [ ] For a moved or renamed document: all six synchronization steps are complete (see “Six
      Synchronization Steps for Moving Documents” below)
- [ ] The language style passes §2 of docs-rules: active voice, declarative statements, present
      tense, consistent terminology, and no marketing language

### Six Synchronization Steps for Moving Documents (Moving = Changing the URL; Every Step Is Required)

1. Move the file with `git mv` (preserving file history)
2. Update site-wide links: grep for the old path and update each occurrence (the build's dead-link
   check provides a safety net, but does not do this for you)
3. Update the sidebar and navigation: `docs/src/.vitepress/config.js` and
   `docs/src/.vitepress/i18n/en.json`
4. Update the en/ mirror: move the corresponding English file in sync
5. Update scripts and configurations that reference the path: `scripts/`, `docs/scripts/`,
   `.github/` workflows, `AGENTS.md`, and `README.md`
6. Update the cache key in `docs/src/.i18n-cache.json` (the key includes the file path)

### Is This a “Patch-Style Fix”? (Documentation D3—Stop at the First Match)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Validation Before Committing (Documentation Changes)

```bash
cd docs
pnpm docs:build                                # Dead links are not tolerated; the build must pass
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Execute documentation examples for real
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1s
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
python ../scripts/ci/check-docs-refs.py        # Referenced paths must resolve (only committed paths count)
```

markdownlint cannot catch VitePress Vue compilation errors (angle-bracket placeholders are treated
as elements, resulting in `Element is missing end tag`), and in documentation-only commits it is
skipped with `no files to check`—**an actual build is the only criterion**. Always put placeholders
in backticks.

If you changed anything under `rfc/`, add one more command:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## III. Measurement (Batch Trees)

Measurements must be performed against a **frozen revision**. Cargo fingerprints include source
paths—**switching trees = a full rebuild, while switching revisions within the same tree = an
incremental build**—so use one tree per batch, always created inside the repository at
`target/yx-scratch/<batch>/tree` (already covered by `.gitignore`). Do not run `git worktree add` /
`git clone` or create a directory outside the repository.

Workflow: `init <batch>` creates the tree (the first full build takes about 5 minutes / uses about 3
GB) → `use <batch> --rev <sha> --by <member> --task <id>` locks the tree and switches revisions →
`build <batch>` performs a unified build (fixed `cargo build --bin yaoxiang-rs`; do not invent
another command) → measure → `release <batch>` unlocks it → finish with `gc` and paste the `status`
output into the task output. See `python scripts/dev/scratch.py --help` for the parameters.

- **Only one holder is allowed at a time**; implementers do not work in this tree (they write code
  in the main repository working tree).
- Only when two people genuinely need to measure different revisions in parallel should you create a
  second tree, `<batch>-b`, and record the reason in the batch record (the cost is another full
  build).
- **`trial` command forms**: `trial <batch> --patch <p> -- <command...>` (recommended; tokens after
  `--` are used verbatim as the command, which may contain `-`/`--` arguments) and
  `trial <batch> <command...> --patch <p>` (usable when the command contains no arguments beginning
  with `-`) are equivalent. `--patch` must appear before `--`; the command must not be empty; mixing
  the two forms (commands on both sides of `--`) is rejected (exit 2).
- **You must run `build` after `trial` before measuring**: trial's mechanical restoration covers
  only source files, while the mutated-version test has already rebuilt the batch-tree
  binary—without another build, all subsequent measurements come from the mutated artifact.
- Probes (`.yx`) also belong in `target/yx-scratch/<batch>/` (`target/` is ignored and is not
  included in corpus differentials); they are workspace artifacts, not documentation sources (see
  AGENTS meta-rule ①).

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, and the review
  checklist (**the canonical code rules**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and wiki-style language style
  (**the canonical documentation rules**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident
  table, external references, and rationale for rejected approaches (diagnostic records, not rule
  sources)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies, and
  parallel groupings
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — Authoritative responsibility-category definitions
  and target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — Overall plan and Decision Register
  D1–D52
