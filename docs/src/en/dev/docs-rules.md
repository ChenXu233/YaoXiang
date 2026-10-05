---
title: 'Documentation Rules (docs-rules)'
description:
  'YaoXiang documentation classification rules (Diátaxis) and wiki-style writing guidelines'
---

# Documentation Rules (docs-rules)

> This document is the **definitive source** for the documentation rules; the pre-work self-check
> entry is [HOWTO.md](./HOWTO.md) (the unified manual; the second section covers modifying
> documentation). This document is at the same level as [coding-rules.md](./coding-rules.md):
> coding-rules governs code; this document governs documentation.

## 0. Three Prohibitions (Documentation Version)

Mirroring the three prohibitions in coding-rules; reviews should also send contributions back on
this basis:

1. **No fabrication**: Before introducing a new term or new document type, first prove it does not
   duplicate an existing concept—check
   [docs/glossary.json](https://github.com/ChenXu233/yaoxiang/blob/main/docs/glossary.json), check
   the classification table in §1, and grep existing documents.
2. **No responsibility accumulation**: A document belongs to exactly one quadrant and serves exactly
   one audience. Stuffing reference tables into tutorials, or writing design rationale into
   references, counts as responsibility accumulation.
3. **Refactor instead of patching**: If the same content is duplicated in ≥2 places, one directory
   mixes two audiences, or a single change touches ≥3 synonymous expressions—stop, adjust the
   structure first, then write the content.

## 1. Classification Rules: Six Top-Level Directories, Six Audiences

The documentation site organizes user documentation according to the four quadrants of
[Diátaxis](https://diataxis.fr/), plus two additional directories aimed at the repository itself:

| Directory      | Quadrant                             | Audience          | The reader's question                                       | Document form                |
| -------------- | ------------------------------------ | ----------------- | ----------------------------------------------------------- | ---------------------------- |
| `tutorial/`    | Tutorial (learning-oriented)         | Beginner          | "Walk me through making my first thing"                     | Step-by-step course          |
| `guide/`       | Guide (task-oriented)                | User              | "I need to do X, how do I do it?"                           | Step-by-step checklist       |
| `reference/`   | Reference (information-oriented)     | User              | "What is the exact meaning of X?"                           | Lookup-style description     |
| `explanation/` | Explanation (understanding-oriented) | Inquirer          | "Why is it designed this way?"                              | Discourse                    |
| `dev/`         | — (outside Diátaxis)                 | Contributor       | "I want to change this repository, what rules do I follow?" | Rules, architecture, process |
| `rfc/`         | — (language governance)              | Language designer | "How was this feature decided, and how do I propose one?"   | Proposals + status catalog   |

`blog/` is time-bound external narrative, `archive/` is a cold store excluded from publishing;
neither participates in the classification.

### Decision Tree

1. Is the reader a **user** of the language or tool?
   - Yes → they want to **learn** (tutorial), to **accomplish** (guide), to **verify** (reference),
     to **understand** (explanation)
   - No → 2
2. Does the reader need to **modify the compiler or repository**? → `dev/`
3. Is the reader **reviewing or submitting a language feature**? → `rfc/`

If a document cannot produce a single answer, it is serving two audiences—split it and place each
part appropriately.

### Easily Confused Boundaries (Empirical)

| Content                                                                                   | Belongs in                    | Common mistake                                              |
| ----------------------------------------------------------------------------------------- | ----------------------------- | ----------------------------------------------------------- |
| CLI usage of `yx check`                                                                   | `reference/check-command.md`  | dev/design/check/                                           |
| Design specification of `yx check` (zero-false-positive principle, incremental mechanism) | `dev/design/check/`           | reference/                                                  |
| Formatter rules (implementation behavior spec)                                            | `dev/design/formatter/`       | reference/                                                  |
| CLI usage of `yx format`                                                                  | `reference/format-command.md` | dev/design/formatter/                                       |
| Language philosophy, manifestos (evergreen discourse)                                     | `explanation/`                | blog/ (blog is time-bound)                                  |
| Compiler architecture and construction schedule                                           | `dev/architecture/`           | explanation/ (it's an operations manual, not discourse)     |
| Feature proposals and ratification records                                                | `rfc/`                        | dev/ (the audience for RFCs is not limited to contributors) |

## 2. Writing Style (Wiki-Style)

This section is inspired by the core spirit of the Wikipedia Manual of Style and applies to all
directories; person and tone are fine-tuned per quadrant (§2.4).

### 2.1 Active Voice Preferred

- Write "The compiler reports E1001", not "E1001 is reported by the compiler".
- Use the passive voice only when the actor is unknown or unimportant.
- Verification: when the sentence contains "is", "by", or "through", ask "who did it". If you can
  answer, change to active voice.

### 2.2 Definitive Statements

- Write "yx check does not generate code", not "yx check may not generate code".
- Facts must be asserted. Uncertain content has only two legitimate ways to be written:
  - Give a definite condition: "When `--watch` is enabled, yx check only re-checks affected files".
  - Delete it, verify, then write it.
- Forbidden hedging words: may, maybe, probably, basically, in some sense. "Should" is allowed only
  to indicate obligation ("You should run the tests"), not to indicate conjecture.

### 2.3 Present Tense

- Write "The parser rejects an empty module", not "The parser will reject an empty module". The
  documentation describes the behavior of the current version.
- Exception: `rfc/draft/` and `rfc/review/` use "will" when describing proposals not yet
  implemented; once a proposal lands, switch back to present tense.

### 2.4 Person

- `reference/`, `explanation/`, `rfc/`: impersonal statements ("This function returns Result").
- `tutorial/`, `guide/`: second-person "you" ("You run yx new").
- `dev/`: rules are impersonal; operational instructions may use "you".
- No directory uses "we": "we recommend" should be written as "recommended".

### 2.5 One Idea Per Sentence

- One sentence asserts one thing; two assertions become two sentences.
- Three or more parallel facts go in a list, not a long sentence.

### 2.6 Terminology Consistency

- One concept has exactly one name across the site. If the earlier text says "ownership", do not
  switch to "proprietorship" later.
- Before introducing a new term, check docs/glossary.json and existing document usage.
- The first occurrence of a term must give a definition or link to a definition.

### 2.7 Objective and Neutral

- Do not write "very simple", "just", "effortless". When readers get stuck, these words feel like
  mockery.
- Do not write marketing words: powerful, elegant, seamless. Replace with verifiable facts:
  "compile-time monomorphization, zero runtime overhead".
- When comparing other languages, state behavioral differences, do not pass judgment.

### 2.8 Verifiability

- Every behavioral assertion must be demonstrable from code or tests; unprovable assertions are not
  allowed.
- Code examples must be executable: CI (`scripts/ci/check-docs-examples.py`) will run them.
  Non-executable examples are marked with `<!-- docs-example: skip -->` and accompanied by a
  justification.

## 3. Structural and Mechanical Rules

- **frontmatter**: `title` is required (the sidebar generation depends on it); `description` is
  recommended.
- **Headings**: One H1 per document; body content starts at H2; do not skip levels.
- **Links**: Internal links always use relative paths to the target file; do not use absolute site
  paths. The build config `ignoreDeadLinks: false` makes broken links fail the build.
- **Moving a file = changing the URL**: You must synchronize references across the site, both
  sidebar configs, the en/ mirror, scripts that reference that path, and AGENTS.md. The full process
  is described in [HOWTO.md](./HOWTO.md), second section "Six Synchronizations for Moving a
  Document".
- **Formatting**: Table alignment and fence blank lines are enforced by prettier
  (`docs/.prettierrc.json`) and markdownlint.
- **zh/en mirroring**: zh is the source; en is synchronized by the translation workflow. After
  adding or moving a zh file, the en-side path must match.

## 4. Responsibility of the Directory Index Page

Each top-level directory's `index.md` answers two questions: "What does this directory accept, and
what does it reject." The index page points to this rule rather than duplicating a hand-maintained
content list—hand-written lists inevitably go stale (the single source of truth for RFC status is
`rfc/TRACKING.md`, generated by script; the same principle applies).
