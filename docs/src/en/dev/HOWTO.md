# Implementer’s Manual (HOWTO)

> **This page is the single entry point for all changes; it is not the source of the rules.** The
> sole authority for the rules is:
>
> - Changing code → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   source of the rules) and
>   [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md) (D1–D52)
> - Changing documentation → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language style)
>
> This page does only two things: **mandatory self-checks before starting work** and **determining
> whether a change is a “patch-style fix.”** If this page conflicts with the source of the rules,
> the source of the rules prevails.

## Step 0: Determine What You Are Changing

| Your change                      | Which section to read | Source of the rules                | Pre-commit validation      |
| -------------------------------- | --------------------- | ---------------------------------- | -------------------------- |
| Changing code (`src/`, `tests/`) | Section One           | [coding-rules.md](coding-rules.md) | See the end of Section One |
| Changing documentation (`docs/`) | Section Two           | [docs-rules.md](docs-rules.md)     | See the end of Section Two |
| Changing both                    | Read both sections    | Follow both sets of rules          | Run both sets of commands  |

Note: Changing code often requires corresponding documentation changes—error-code tables, CLI
options, and user-visible behavior all have assertions in the documentation. Passing Section One’s
acceptance criteria does not mean you are done. First answer: “Where is this behavior documented,
and does it need to change as well?”

## I. Changing Code

### Which Stage Are You At?

Overall execution table: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find your
level-3 task entry** (such as 4.2.7) and confirm its: whether its prerequisites are complete, which
acceptance-criterion level it uses (C1–C6; the criteria are defined in
[07](architecture/07-equivalence-oracle.md)), and whether it is marked as an “exclusive commit.”

### Pre-work Self-check (Complete Every Item Before Writing Code)

- [ ] I read the full level-3 entry for this task in [09](architecture/09-execution-wbs.md), rather
      than only the task title
- [ ] I read the **actual context** of the code being changed (I opened the file rather than
      imagining it from the task description)
- [ ] I know this task’s acceptance-criterion level (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavioral equivalence / C5 AST + diagnostics + behavior / C6 deletion
      only)
- [ ] I identified the **error category (A–F, according to RFC-039 D59)**: A/B/C categories (the old
      behavior was **unsuccessful**—crash/stack overflow, silent failure, silent mismatch, or
      silently returning the wrong answer) may be fixed incidentally during refactoring, but they
      must **fail before the fix and pass afterward** without changing the externally observable
      behavior of programs that previously succeeded; D/E/F categories (user-visible semantics /
      external contract tables / RFC-level design) require a user decision first (see
      [07 §Semantic Correction Routing Criteria](architecture/07-equivalence-oracle.md))
- [ ] I ran **D0**: Does this change touch any table (error codes/opcodes/types/stages)? Which
      module is the authoritative source? If you cannot name the module, you have not checked
- [ ] I ran **D1/D2**: Can newly added types/enums/constant tables be expressed using existing
      concepts plus parameters? Do they have the same semantics as existing concepts? (Import
      aliases must not be used to paper over semantic differences)
- [ ] I ran **D4**: Does the new code belong to an **existing responsibility category** in the
      target module? (See the directory responsibility table in
      [01-routing.md](architecture/01-routing.md) for the authoritative definition of responsibility
      categories)
- [ ] I ran the **trade-off criterion**: Is my rejection of the proposal (or the proposal author’s
      rejection) based solely on **correctness** and **readability**? Rejecting a proposal because
      it has a “large change surface / requires a new mechanism / is expensive” = an invalid
      rejection, and review will send it back (coding-rules, Part Two); for trade-offs requiring a
      user decision, provide all four items in accordance with the repository-root `AGENTS.md`
- [ ] Is my new mechanism a “register–consume” mechanism (one party registers data and another
      consumes it)? If so, are the end-to-end tests that **directly drive** both the producer and
      consumer delivered in the same batch? A consumer test that manually constructs registered data
      is a silent channel, and review will send it back (coding-rules, Part Six; precedent: RFC-039
      Decision Registry D55)
- [ ] If this task involves writing or changing tests: I read
      [test-specification.md](test-specification.md)—the `test_<what>_<scenario>` naming prefix, AAA
      three-part comments for blocks longer than five lines, custom assertion messages, enum
      matching with `assert!(matches!(...))`, no `use super::*`, and no inline `mod tests {}`
- [ ] If my change affects IR or corpus execution behavior (frontend/ir_gen/std/corpus `.yx`): I
      plan to update the IR snapshots and corpus differential baselines **in the same commit** (see
      the common-commands section of AGENTS.md), and manually review both diffs—parallel workstream
      #385 once failed to synchronize its snapshots and was blocked by the gate (precedent:
      dabdfd96)
- [ ] If the task says “fail before the fix, then pass” (such as 2.4.1 and 8.8.1), I have confirmed
      that the failing criterion already exists and currently fails
- [ ] I have not prepared to “delete failing tests” or “relax the criteria” just to pass
      acceptance—**being unable to satisfy them is an implementation defect and must be reported
      honestly, not a reason to relax them** (D24/no C5′)
- [ ] I have asked myself where this behavior is documented and whether the documentation must
      change with this change (change → Section Two)

### Is This a “Patch-Style Fix”? (D3—If You Hit One, Stop)

```
1) 同一行为需要在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层，禁止复制
2) 需要新增"第 6 个入口/接线点"？    → 是 → ⛔ 禁止手工接线，改为登记进阶段表
3) 一次修改要同步改 ≥3 处同义映射？  → 是 → ⛔ 这是架构变更，停工，走设计文档流程
4) 以上皆否                          → 允许局部补丁，但必须附带回归测试
```

**Rule 3 is the reason this manual exists.** “Changing ≥3 synonymous mappings” means you are
maintaining a parallel source of truth—the correct action is to eliminate that parallelism, not add
another instance. Historical incidents: three sets of operator enumerations, five compiler entry
points, and a hand-written table of type synonyms (see the incident table in
[08](architecture/08-maintenance-mechanism.md)).

### Several Rules That Are Easy to Violate

- **Do not change code you have not inspected**: If you have not opened the file or searched for
  reference sites, you may not modify it. Line numbers reflect the current state of the repository;
  documentation line numbers drift.
- **Do not leave deferred implementations**: Core functionality must not contain `todo!()` /
  `Vec::new() // Not implemented yet` / “fix it later in a separate issue.” If you find one, fix it
  or register it as a task in the current phase—do not introduce new deferred work (D52 precedent:
  all three data-loss instances in `.42` have been brought under P7).
- **Verification items must be checked line by line**: When a task is marked “must be verified line
  by line; equivalence must not be assumed” (such as 4.2.7 and 6.1.3), write the verification result
  in the PR description.
- **Deleting tests to get a green build = a violation**: The DoD requires that the number of tests
  at the end of each phase must not be lower than it was at the beginning.
- **Do not manually check problems already covered by tools**: fmt / clippy / `build.rs` gates /
  `scripts/ci/check-*.py` from P0 onward. Review should focus only on what machines cannot check
  (responsibility classification and D3).

### What If You Get Stuck?

| Situation                                                       | Correct action                                                                                                                                                                | Incorrect action                                           |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| The criteria do not pass                                        | Report honestly and fix the implementation; if you suspect the criteria are wrong, return to [07](architecture/07-equivalence-oracle.md) and propose a change to the criteria | Add a whitelist or exemption for the criteria              |
| You discover that a design premise does not hold                | Return to the [RFC-039 Decision Registry](../rfc/accepted/039-compiler-architecture.md), amend the decision, and explain why                                                  | Bypass the decision and proceed quietly                    |
| The task is larger than expected                                | Break the level-3 task down further in [09](architecture/09-execution-wbs.md)                                                                                                 | Mix multiple acceptance criteria into one oversized commit |
| You find a contradiction between the documentation and the code | Trust the code first, then fix the documentation (precedent: the layer order in `layers/README.md` was reversed, D7)                                                          | Write code according to incorrect documentation            |

### Pre-commit Validation (Changing Code)

```bash
cargo fmt                                              # Format (required)
cargo clippy --all --all-features --all-targets -- -D warnings
python scripts/dev/testsuite.py run fast               # Required before commit: lib + integration side, see below
python scripts/ci/check-test-channel.py                # One source of truth across four locations (this page / AGENTS.md / test-specification / pre-commit)
```

The target set for `run fast` is explicit: `--lib` (unit tests under src/) plus four integration
targets: `integration` / `formatter_integration` / `yx_runner` / `yx_multifile_runner`; the first
stage of `pre-commit` runs this command, and the second stage runs `cargo test --all --quiet` to
block the commit. Doctests (`cargo test --doc`) and corpus differential testing are not covered by
`run fast`; CI handles them. An exclusive wave that does not fit the budget will list every “skipped
N tests” item at the end and provide a fallback path; it will never skip tests silently.

## II. Changing Documentation

### Classification: Which Directory Does This Document Belong To?

Use the decision tree in §1 of [docs-rules.md](docs-rules.md). The answer must be unique (tutorial /
guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this document
serves two audiences—split it before making changes.**

### Pre-work Self-check (Complete Every Item Before Changing Documentation)

- [ ] I used the §1 decision tree in docs-rules to determine the directory, and the answer is unique
- [ ] I searched for similar content; the same content does not have ≥2 copies (if it does, merge
      them rather than adding a third copy)
- [ ] I checked docs/glossary.json and the existing documentation, reused existing terminology, and
      did not invent new names
- [ ] New documentation: the frontmatter `title` has been added; the sidebar entry has been added
      (`config.js` + `i18n/en.json`; generateSidebar scans files within the directory, so no manual
      entry is needed)
- [ ] Moved or renamed documentation: all six synchronization steps are complete (see “Six
      Synchronization Steps for Moving Documentation” below)
- [ ] The language style passes §2 of docs-rules: active voice, declarative sentences, present
      tense, consistent terminology, and no marketing language

### Six Synchronization Steps for Moving Documentation (Moving = Changing the URL; None May Be Omitted)

1. Use `git mv` to move the file (preserving file history)
2. Update site-wide links one by one by searching for the old path (the build’s dead-link check
   provides a backstop, but it does not do this for you)
3. Update the sidebar and navigation: `docs/src/.vitepress/config.js` and
   `docs/src/.vitepress/i18n/en.json`
4. Mirror the change in en/: move the corresponding English file at the same time
5. Update scripts and configurations that reference the path: `scripts/`, `docs/scripts/`,
   `.github/` workflows, `AGENTS.md`, and `README.md`
6. Update the cache keys in `docs/src/.i18n-cache.json` (the keys contain file paths)

### Is This a “Patch-Style Fix”? (Documentation Version of D3—If You Hit One, Stop)

```
1) 同一内容要改到第 2 处副本？                    → 是 → ⛔ 合并副本，只留一处
2) 要给一个目录新增第 N 个"例外说明"？           → 是 → ⛔ 这是分类问题，先调结构
3) 读者要在两个顶级目录间来回跳转才能凑齐一件事？ → 是 → ⛔ 拆分或合并，先调结构
4) 以上皆否                                        → 允许局部修改
```

### Pre-commit Validation (Changing Documentation)

```bash
cd docs
pnpm docs:build                                # Zero tolerance for dead links; the build must pass
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Execute documentation examples for real
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1s
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
```

If you changed `rfc/`, add one more command:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, and the review
  checklist (**the authoritative code rules**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules and wiki-style language style
  (**the authoritative documentation rules**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident
  table, external references, and rationale for rejected proposals (diagnostic record, not a source
  of rules)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies, and
  parallel work groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — Definitions of the C1–C6
  criteria
- [01-routing.md](architecture/01-routing.md) — Authoritative definitions of responsibility
  categories and the target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — General outline and decision registry
  D1–D52
