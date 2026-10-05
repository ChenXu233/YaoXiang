# Implementer's Handbook (HOWTO)

> **This page is the unified entry point for all changes, not the rules themselves.** The sole
> authority for the rules is:
>
> - For code changes → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the exact
>   rules) and [RFC-039 Decision Register](../rfc/draft/039-compiler-architecture.md) (D1–D52)
> - For doc changes → [docs-rules.md](docs-rules.md) (Diátaxis classification rules + wiki-style
>   language)
>
> This page does only two things: **mandatory self-check before starting work** and **determining
> whether a change is a "patch-style fix"**. If this page conflicts with the rules, the rules take
> precedence.

## Step 0: Determine What You're Changing

| Your change             | Read section | Rules                              | Verify before commit      |
| ----------------------- | ------------ | ---------------------------------- | ------------------------- |
| Code (`src/`, `tests/`) | Section 1    | [coding-rules.md](coding-rules.md) | See end of section 1      |
| Docs (`docs/`)          | Section 2    | [docs-rules.md](docs-rules.md)     | See end of section 2      |
| Both                    | Both         | Both                               | Run both sets of commands |

Note: Code changes often must update docs in sync — error code tables, CLI arguments, and
user-visible behavior are all asserted in docs. Passing the section 1 acceptance does not mean
you're done; first answer "where in the docs is this behavior described, and should it be updated
too".

## I. Changing Code

### Which Stage You're In

Master execution table: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find your
level-3 task entry** (e.g., 4.2.7) and confirm: whether its prerequisites are complete, which
acceptance level it is — C1–C6 (level definitions in [07](architecture/07-equivalence-oracle.md)),
and whether it's marked as "exclusive commit".

### Pre-work Self-check (Go Through All of These Before Writing Code)

- [ ] I have read the original level-3 entry for this task in
      [09](architecture/09-execution-wbs.md), not just the task title
- [ ] I have read the **actual context** of the code being changed (opened the file and looked, not
      imagined from the task description)
- [ ] I know this task's acceptance level (C1 zero-diff / C2 diagnostic set / C3 diagnostic codes /
      C4 behavioral equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have run **D0**: Does this change touch any table (error codes / opcodes / types / stages)?
      Which module is the authoritative source? — If you can't name the module, you haven't checked
- [ ] I have run **D1/D2**: Can the newly added type / enum / constant table be expressed with
      existing concepts + parameters? Is it semantically the same as an existing concept? (Same
      semantics forbids bridging with import aliases)
- [ ] I have run **D4**: Does the new code belong to the **existing responsibility category** of the
      target module? (Authoritative definition of responsibility categories in
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] If the task says "red first, then green" (e.g., 2.4.1, 8.8.1), I have confirmed the red
      criterion exists and the test is currently red
- [ ] I have not prepared to "delete failing tests" or "loosen criteria" to pass acceptance —
      **inability to do so is an implementation defect, report as-is, not a reason to loosen**
      (D24/no C5′)
- [ ] I have asked myself: which docs describe this behavior, and does this change need to update
      them too (change → Section 2)

### Is This a "Patch-style Fix"? (D3, stop if any match)

```
1) The same behavior needs to be duplicated in ≥2 places? → Yes → ⛔ Must be promoted to a shared layer, duplication forbidden
2) Need to add a "6th entry/wiring point"?                → Yes → ⛔ Manual wiring forbidden, register it in the stage table
3) One change must update ≥3 synonymous mappings in sync?  → Yes → ⛔ This is an architecture change, stop, follow the design-doc process
4) None of the above                                       → Local patch allowed, but must come with regression tests
```

**Rule 3 is the reason this handbook exists.** "Changing ≥3 synonymous mappings" means you're
maintaining a parallel source of truth — the correct action is to eliminate the parallel, not add
another one. Historical accidents: 3 sets of operator enums, 5 compilation entry points,
hand-written type synonym tables (see [08](architecture/08-maintenance-mechanism.md) accident table
for details).

### A Few Hard Rules That Are Easy to Trip Over

- **Don't change code you haven't read**: Without opening the file and grepping the reference
  points, you may not touch it. Line numbers are based on the current repo state; doc line numbers
  will drift.
- **No "to be implemented" leftovers**: Core features must not have `todo!()` /
  `Vec::new() // Not implemented yet` / "independent issue to fix later". If found, fix it, or
  register it as a task in the current stage — no new leftovers allowed (D52 precedent: `.42`
  three-place data loss has all been absorbed into P7).
- **Items marked "verify line by line" must be verified line by line**: When a task is marked "must
  verify line by line, cannot assume equivalence" (e.g., 4.2.7, 6.1.3), write the verification
  conclusion into the PR description.
- **Deleting tests to get a green light = violation**: DoD requires the test count at the end of
  each stage to be no less than at the start.
- **Mechanical problems already caught by tooling — don't check by hand**: fmt / clippy / `build.rs`
  gates / from P0 onward, `scripts/ci/check-*.py`. Review only what machines can't check
  (responsibility judgment, D3).

### What to Do When Stuck

| Situation                           | Correct action                                                                                                                                        | Wrong action                                        |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Criterion won't go green            | Report as-is, fix the implementation; suspecting the criterion is wrong → go back to [07](architecture/07-equivalence-oracle.md) and propose a change | Add a whitelist/exception to the criterion          |
| Found a design premise doesn't hold | Go back to [RFC-039 Decision Register](../rfc/draft/039-compiler-architecture.md), change the decision and explain why                                | Bypass the decision and do it quietly               |
| Task is larger than expected        | In [09](architecture/09-execution-wbs.md), break the level-3 task into finer pieces                                                                   | One huge commit mixing multiple acceptance criteria |
| Found docs and code contradict      | Trust the code first, then fix the docs (precedent: `layers/README.md` layer order was reversed, D7)                                                  | Write code following the wrong docs                 |

## II. Changing Docs

### Classification: Which Directory Does This Doc Belong To?

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you can't give a unique answer, this
doc is serving two audiences — split first, then start.**

### Pre-work Self-check (Go Through All of These Before Changing Docs)

- [ ] I used the docs-rules §1 decision tree to determine the directory; the answer is unique
- [ ] I grepped for similar content; the same content has no ≥2 copies (if it does, merge, don't add
      a 3rd)
- [ ] I checked docs/glossary.json and existing docs, used existing terms, didn't invent new names
- [ ] New doc: frontmatter `title` is written; sidebar is mounted (`config.js` + `i18n/en.json`;
      generateSidebar scans files in the directory and needs no manual mounting)
- [ ] Move/rename doc: all six syncs completed (see "Six Syncs for Moving Docs" below)
- [ ] Language style has passed docs-rules §2: active voice, declarative statements, present tense,
      consistent terminology, no marketing words

### Six Syncs for Moving Docs (Moving = Changing URL, every one is required)

1. Move with `git mv` (preserve file history)
2. Site-wide links: grep the old path and update one by one (the build's dead-link check will catch
   misses, but it doesn't do it for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: move the corresponding English file in sync
5. Scripts and configs that reference this path: `scripts/`, `docs/scripts/`, `.github/` workflows,
   `AGENTS.md`, `README.md`
6. The cache key in `docs/src/.i18n-cache.json` (key contains file path)

### Is This a "Patch-style Fix"? (Docs version of D3, stop if any match)

```
1) The same content needs to be changed in a 2nd copy?                            → Yes → ⛔ Merge copies, keep only one
2) Need to add an Nth "exception note" to a directory?                            → Yes → ⛔ This is a classification problem, adjust the structure first
3) Readers have to jump between two top-level directories to assemble one thing? → Yes → ⛔ Split or merge, adjust the structure first
4) None of the above                                                              → Local modification allowed
```

### Verification Before Commit (Doc Changes)

```bash
cd docs
pnpm docs:build                                # zero dead links, build must be green
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # execute doc examples for real
python ../scripts/ci/check-docs-orphan.py      # orphan pages and duplicate H1
python ../scripts/ci/check-docs-truth.py       # version injection and fact reconciliation
```

If you changed `rfc/`, add one more:

```bash
python ../scripts/rfc/check_tracking.py        # RFC status consistency
```

## References

- [coding-rules.md](coding-rules.md) — three prohibitions, D0–D4, red lines, review checklist
  (**code rules proper**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language (**doc rules
  proper**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — historical accident
  table, external references, rejected scheme reasoning (diagnostic record, not a rule source)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — level-3 task table, dependencies,
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 acceptance definitions
- [01-routing.md](architecture/01-routing.md) — authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/draft/039-compiler-architecture.md) — master plan and decision register D1–D52
