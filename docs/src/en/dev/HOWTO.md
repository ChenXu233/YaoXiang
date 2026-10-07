# Implementer's Handbook (HOWTO)

> **This page is the unified entry point for all changes, not the rules themselves.** The sole
> authoritative source of the rules:
>
> - For code changes → [coding-rules.md](coding-rules.md) (three prohibitions + D0–D4, the precise
>   rules themselves) and [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md)
>   (D1–D52)
> - For documentation changes → [docs-rules.md](docs-rules.md) (Diátaxis classification rules +
>   wiki-style language)
>
> This page does only two things: **mandatory pre-work self-check** and **determining whether a
> change is a "patch-style fix"**. If this page conflicts with the rules, the rules prevail.

## Step 0: Determine What You Are Changing

| Your change                      | Read section | Rules                              | Pre-commit verification |
| -------------------------------- | ------------ | ---------------------------------- | ----------------------- |
| Changing code (`src/`, `tests/`) | Section 1    | [coding-rules.md](coding-rules.md) | See end of Section 1    |
| Changing documentation (`docs/`) | Section 2    | [docs-rules.md](docs-rules.md)     | See end of Section 2    |
| Changing both                    | Both         | Both                               | Run both command sets   |

Note: Changing code often requires syncing documentation — error code tables, CLI parameters, and
user-visible behavior are all asserted in documentation. Passing Section 1 acceptance is not the end
— first answer "where in the documentation is this behavior described, and does it need to be
updated".

## 1. Changing Code

### What Stage Are You At

Construction master schedule: [09-execution-wbs.md](architecture/09-execution-wbs.md). **First find
your level-3 task entry** (e.g. 4.2.7), and confirm its: prerequisites completed, acceptance
criterion level (C1–C6, see [07](architecture/07-equivalence-oracle.md) for definitions), and
whether it is marked "exclusive commit".

### Pre-work Self-check (Run through all of these, before writing code)

- [ ] I have read the level-3 entry text in [09](architecture/09-execution-wbs.md) for this task,
      not just the task title
- [ ] I have read the **actual context** of the code being changed (opened the file, not imagined
      from the task description)
- [ ] I know the acceptance criterion level for this task (C1 zero-diff / C2 diagnostic set / C3
      diagnostic code / C4 behavior equivalence / C5 AST+diagnostics+behavior / C6 pure deletion)
- [ ] I have run **D0**: does this change touch any table (error code / opcode / type / stage)?
      Which is the authoritative source module? — If you cannot name the module, you have not
      checked
- [ ] I have run **D1/D2**: can the new type/enum/constant table be expressed with existing
      concepts + parameters? Is it semantically equivalent to existing concepts? (Same semantics
      forbids bridging with `use` aliases)
- [ ] I have run **D4**: does the new code fall within the **existing responsibility category** of
      the target module? (Authoritative definition of responsibility categories in
      [01-routing.md](architecture/01-routing.md) directory responsibility table)
- [ ] Is my new mechanism of the "register-consume" type (one side registers data, the other
      consumes)? If so, are the **direct-driven** end-to-end tests on both producer and consumer
      delivered in the same batch? Hand-constructing registration data in consumer tests = silent
      channel, reject in review (coding-rules Part 6; precedent see RFC-039 Decision Register D55)
- [ ] If this task requires writing/changing tests: I have read
      [test-specification.md](test-specification.md) — naming `test_<what>_<scenario>` prefix, AAA
      three-segment comments over 5 lines, assertions with custom messages, enum matching
      `assert!(matches!(...))`, no `use super::*`, no inline `mod tests {}`
- [ ] If my change will alter IR or corpus runtime behavior (frontend / ir_gen / std / corpus
      `.yx`): I plan to **commit in the same submission** the IR snapshots and corpus diff baselines
      (see AGENTS.md common commands section), and manually review both diffs — parallel stream #385
      once missed syncing snapshots and was blocked by the gate (precedent dabdfd96)
- [ ] If the task says "red first, then green" (e.g. 2.4.1, 8.8.1), I confirm the red criterion
      exists and is currently red
- [ ] I have not prepared to "delete failing tests" or "loosen criteria" to pass acceptance —
      **inability to do so is an implementation defect, report it as is, not a reason to loosen**
      (D24 / no such C5′)
- [ ] I have asked myself: where in the documentation is this behavior described, and does this
      change need to update the documentation (yes → Section 2)

### Is This a "Patch-style Fix" (D3, stop if hit)

```
1) The same behavior needs to be duplicated in ≥2 places?        → Yes → ⛔ Must be lifted to the shared layer, duplication forbidden
2) Need to add a "6th entry/wiring point"?                      → Yes → ⛔ No manual wiring, change to registering into the stage table
3) One change needs to sync ≥3 synonymous mappings?             → Yes → ⛔ This is an architecture change, stop work, go through design doc process
4) None of the above                                              → Local patch allowed, but must include regression tests
```

**Item 3 is the reason this handbook exists.** "Changing ≥3 synonymous mappings" means you are
maintaining a parallel source of truth — the correct action is to eliminate the parallel, not add
another. Historical incidents: 3 sets of operator enums, 5 compilation entry points, hand-written
type synonym tables (see [08](architecture/08-maintenance-mechanism.md) incident table for details).

### A Few Hard Rules That Are Easy to Trip On

- **Don't change code you haven't read**: no opening the file, no grep of reference points, no
  touching it. Line numbers are based on the repository as it is; documentation line numbers will
  drift.
- **No "to be implemented" leftovers**: core functionality must not have `todo!()` /
  `Vec::new() // Not implemented yet` / "separate issue to fix later". If found, fix them, or
  register them into the current stage's tasks — must not introduce new leftovers (D52 precedent:
  `.42` three data losses all consolidated into P7)
- **Verification items must be verified line by line**: when the task is marked "must verify line by
  line, cannot assume equivalence" (e.g. 4.2.7, 6.1.3), write the verification conclusion into the
  PR description
- **Deleting tests to get a green light = violation**: DoD requires that the number of tests at the
  end of each stage is not less than at the start
- **For mechanical issues already covered by tools, don't manually check**: fmt / clippy /
  `build.rs` gate / `scripts/ci/check-*.py` from P0. Review only looks at what machines cannot check
  (responsibility determination, D3)

### What to Do When Stuck

| Situation                      | Correct action                                                                                                                                                   | Wrong action                                        |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Criterion won't go green       | Report as is, fix the implementation; suspect the criterion is wrong → go back to [07](architecture/07-equivalence-oracle.md) and propose changing the criterion | Add whitelist/exemption to the criterion            |
| Design premise found invalid   | Go back to [RFC-039 Decision Register](../rfc/accepted/039-compiler-architecture.md), change the decision and explain why                                        | Circumvent the decision and do it quietly           |
| Task larger than expected      | In [09](architecture/09-execution-wbs.md), break the level-3 task down further                                                                                   | One huge commit mixing multiple acceptance criteria |
| Documentation contradicts code | Trust the code first, then fix the documentation (precedent: `layers/README.md` layer order was reversed, D7)                                                    | Write code following the wrong documentation        |

## 2. Changing Documentation

### Classification: Which Directory Does This Document Belong To

Use the decision tree in [docs-rules.md](docs-rules.md) §1 to answer; the answer must be unique
(tutorial / guide / reference / explanation / dev / rfc). **If you cannot give a unique answer, this
document serves two audiences — split first, then act.**

### Pre-work Self-check (Run through all of these, before changing documentation)

- [ ] I have used the docs-rules §1 decision tree to determine the directory, and the answer is
      unique
- [ ] I have grepped for similar content; the same content does not have ≥2 copies (if so, merge, do
      not add a 3rd)
- [ ] I have checked `docs/glossary.json` and existing documentation, and use existing terms, not
      making up new names
- [ ] New documentation: frontmatter `title` written; sidebar hooked up (`config.js` +
      `i18n/en.json`; generateSidebar scanning files in the directory does not require manual
      mounting)
- [ ] Move/rename document: all six syncs completed (see "Six Syncs for Moving Documents" below)
- [ ] Language style has passed docs-rules §2: active voice, definitive statements, present tense,
      consistent terminology, no marketing words

### Six Syncs for Moving Documents (Move = change URL, every one is required)

1. Move with `git mv` (preserve file history)
2. Site-wide links: grep the old path and update each one (the build's dead-link check will catch
   it, but does not do it for you)
3. Sidebar and navigation: `docs/src/.vitepress/config.js` and `docs/src/.vitepress/i18n/en.json`
4. en/ mirror: synchronously move the corresponding English file
5. Scripts and configs referencing the path: `scripts/`, `docs/scripts/`, `.github/` workflows,
   `AGENTS.md`, `README.md`
6. `docs/src/.i18n-cache.json` cache keys (keys contain the file path)

### Is This a "Patch-style Fix" (Documentation D3, stop if hit)

```
1) The same content needs to be changed in a 2nd copy?                          → Yes → ⛔ Merge the copies, keep only one
2) Need to add the Nth "exception note" to a directory?                        → Yes → ⛔ This is a classification problem, adjust structure first
3) Readers need to jump between two top-level directories to complete one thing? → Yes → ⛔ Split or merge, adjust structure first
4) None of the above                                                              → Local modification allowed
```

### Pre-commit Verification (For documentation)

```bash
cd docs
pnpm docs:build                                # Zero tolerance for dead links, build must be green
pnpm lint:md                                   # markdownlint
pnpm format:md:check                           # prettier
python ../scripts/ci/check-docs-examples.py    # Document examples actually executed
python ../scripts/ci/check-docs-orphan.py      # Orphan pages and duplicate H1
python ../scripts/ci/check-docs-truth.py       # Version injection and fact reconciliation
```

If you changed `rfc/`, add one more:

```bash
python ../scripts/rfc/check_tracking.py        # RFC state consistency
```

## References

- [coding-rules.md](coding-rules.md) — Three prohibitions, D0–D4, red lines, review checklist (**the
  code rules themselves**)
- [docs-rules.md](docs-rules.md) — Diátaxis classification rules, wiki-style language (**the
  documentation rules themselves**)
- [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) — Historical incident
  table, external references, rejected proposal arguments (diagnostic record, not a source of rules)
- [09-execution-wbs.md](architecture/09-execution-wbs.md) — Level-3 task table, dependencies,
  parallel groups
- [07-equivalence-oracle.md](architecture/07-equivalence-oracle.md) — C1–C6 criterion definitions
- [01-routing.md](architecture/01-routing.md) — Authoritative definition of responsibility
  categories, target directory structure
- [RFC-039](../rfc/accepted/039-compiler-architecture.md) — Master plan and decision register D1–D52
