---
title: 'Documentation Rules (docs-rules)'
description:
  'Classification rules (Diátaxis) and wiki-style language guidelines for YaoXiang documentation'
---

# Documentation Rules (docs-rules)

> This document is the **source of truth** for documentation rules; before making changes, use
> [HOWTO.md](./HOWTO.md) (the unified manual; Section 2 covers documentation changes) as the
> preflight entry point. This document is at the same level as [coding-rules.md](./coding-rules.md):
> coding-rules governs code, whereas this document governs documentation.

## 0. Three Prohibitions (Documentation Edition)

These mirror the three prohibitions in coding-rules; reviews are rejected under the same rules:

1. **Do not invent**: Before introducing a new term or documentation type, first demonstrate that it
   does not duplicate an existing concept—check
   [docs/glossary.json](https://github.com/ChenXu233/yaoxiang/blob/main/docs/glossary.json), consult
   the classification table in §1, and grep the existing documentation.
2. **Do not accumulate responsibilities**: A document belongs to only one quadrant and serves only
   one audience. Putting a reference table in a tutorial or design rationale in a reference
   accumulates responsibilities.
3. **Refactor, don't patch**: If the same content is copied in ≥2 places, a directory contains a mix
   of two audiences, or one change affects ≥3 synonymous statements—stop, adjust the structure
   first, then write the content.

## 1. Classification Rules: Six Top-Level Directories, Six Audiences

In addition to the four quadrants of user documentation organized according to
[Diátaxis](https://diataxis.fr/), the documentation site includes two directories aimed at the
repository itself:

| Directory      | Quadrant                             | Audience           | Reader's question                                              | Document form                  |
| -------------- | ------------------------------------ | ------------------ | -------------------------------------------------------------- | ------------------------------ |
| `tutorial/`    | Tutorial (learning-oriented)         | Beginners          | “Take me through building my first thing”                      | Progressive course             |
| `guide/`       | Guide (task-oriented)                | Users              | “I need to complete X; how do I do that?”                      | Checklist                      |
| `reference/`   | Reference (information-oriented)     | Users              | “What exactly does X mean?”                                    | Lookup-oriented description    |
| `explanation/` | Explanation (understanding-oriented) | Curious readers    | “Why is it designed this way?”                                 | Exposition                     |
| `dev/`         | — (outside Diátaxis)                 | Contributors       | “I want to modify this repository; which rules must I follow?” | Rules, architecture, workflows |
| `rfc/`         | — (language governance)              | Language designers | “How is this feature decided and how is it proposed?”          | Proposal + status directory    |

`blog/` is a time-based external narrative, and `archive/` is a cold-storage area excluded from
publication; neither participates in the classification.

### Classification Decision Tree

1. Is the reader a **user** of the language or tool?
   - Yes → Do they want to **learn** (tutorial), **accomplish a task** (guide), **look something
     up** (reference), or **understand** (explanation)?
   - No → 2
2. Does the reader want to **modify the compiler or repository**? → `dev/`
3. Is the reader **reviewing or submitting a language feature**? → `rfc/`

If a document cannot yield a unique answer, it is serving two audiences—split it and place each part
in its proper location.

### Ambiguous Boundaries (Empirical)

| Content                                                                             | Home                          | Common misplacement                                          |
| ----------------------------------------------------------------------------------- | ----------------------------- | ------------------------------------------------------------ |
| Command-line usage of `yx check`                                                    | `reference/check-command.md`  | dev/design/check/                                            |
| Design specifications for `yx check` (zero false positives, incremental processing) | `dev/design/check/`           | reference/                                                   |
| Formatting rule wording (implementation behavior specification)                     | `dev/design/formatter/`       | reference/                                                   |
| Command-line usage of `yx format`                                                   | `reference/format-command.md` | dev/design/formatter/                                        |
| Language philosophy and manifesto (evergreen discourse)                             | `explanation/`                | blog/ (blog is time-based)                                   |
| Compiler architecture and implementation plan                                       | `dev/architecture/`           | explanation/ (it is an operations manual, not an exposition) |
| Feature proposals and final decision records                                        | `rfc/`                        | dev/ (the RFC audience extends beyond contributors)          |

## 2. Language Style (Wiki-Style Writing)

This section draws on the core principles of Wikipedia's style manual and applies to every
directory; grammatical person and tone are adjusted slightly according to the quadrant (§2.4).

### 2.1 Prefer the Active Voice

- Write “The compiler reports E1001,” not “E1001 is reported by the compiler.”
- Use the passive voice only when the actor is unknown or unimportant.
- Test: when a sentence contains a passive construction, ask “Who did it?” If you can answer,
  rewrite it in the active voice.

### 2.2 Make Definite Assertions

- Write “`yx check` does not generate code,” not “`yx check` may not generate code.”
- Facts must be stated definitively. Uncertain content has only two legitimate forms:
  - State definite conditions: “When `--watch` is enabled, `yx check` rechecks only affected files.”
  - Remove it, verify it, then write it.
- Ban vague terms: possibly, perhaps, probably, basically, to some extent. Use “should” only to
  express obligation (“You should run the tests”), never speculation.

### 2.3 Use the Present Tense

- Write “The parser rejects empty modules,” not “The parser will reject empty modules.”
  Documentation describes the behavior of the current version.
- Exception: `rfc/draft/` and `rfc/review/` use “will” when describing proposals that have not yet
  been implemented; after a proposal is implemented, switch back to the present tense.

### 2.4 Grammatical Person

- `reference/`, `explanation/`, `rfc/`: impersonal statements (“The function returns Result”).
- `tutorial/`, `guide/`: second-person “you” (“You run yx new”).
- `dev/`: rule statements are impersonal; operational instructions may use “you.”
- No directory uses “we”: rewrite “We recommend” as “Recommended.”

### 2.5 One Idea per Sentence

- A sentence makes only one assertion; if it makes two, write two sentences.
- Use a list for three or more parallel facts rather than a long sentence.

### 2.6 Consistent Terminology

- A concept has only one name across the entire site. If earlier text says “ownership,” later text
  must not switch to a term such as “proprietary relationship.”
- Before introducing a term, check docs/glossary.json and how it is used in the existing
  documentation.
- On a term's first occurrence, define it or link to its definition.

### 2.7 Objective Neutrality

- Do not write “very simple,” “only need to,” or “effortless.” When a reader gets stuck, such words
  are taunts.
- Do not use marketing terms: powerful, elegant, seamless. Replace these with verifiable facts:
  “Compile-time monomorphization, with no runtime overhead.”
- When comparing other languages, state behavioral differences rather than judging their merits.

### 2.8 Verifiability

- Every assertion about behavior must be substantiated by code or a test; do not make assertions you
  cannot substantiate.
- Code examples must be executable: CI (`scripts/ci/check-docs-examples.py`) runs them. Mark any
  example that cannot run with `<!-- docs-example: skip -->` and provide a reason.

## 3. Structural and Mechanical Rules

- **frontmatter**: `title` is required (sidebar generation depends on it); `description` is
  recommended.
- **Headings**: one H1 per document; the body starts at H2; do not skip levels.
- **Links**: All internal links must use a relative path to the target file, not a site-absolute
  path. The build configuration `ignoreDeadLinks: false` means dead links fail the build.
- **Do not include line numbers**: Line numbers drift whenever content changes, while
  `check-docs-refs.py` only validates whether the path resolves. Locate content semantically (“the
  arm in `ir_gen` that retrieves the `Expr::Cast` span”); when line numbers are genuinely required,
  retrieve them from the referenced revision (`git show <rev>:<file>`). **Exception**: error-code
  reference pages generated at build time include `file:line` emission-point anchors—the generator
  computes line numbers from the current source during the build, so they are always up to date.
- **Moving a file = changing the URL**: You must synchronize all site-wide references, both sidebar
  configurations, the en/ mirror, any scripts that reference the path, and `AGENTS.md`. For the full
  procedure, see [HOWTO.md](./HOWTO.md), Section 2, “Six Synchronizations When Moving a Document.”
- **Formatting**: prettier (`docs/.prettierrc.json`) and markdownlint enforce table alignment and
  blank lines around fenced blocks.
- **Chinese-English mirror**: zh is the source, and en is synchronized through the translation
  workflow. After adding or moving a zh file, the corresponding path in en must match.
  **Exception**: error-code reference pages are not checked into the repository; both the zh and en
  versions are generated at build time by `scripts/docs/gen-error-code-docs.py` (with text sourced
  from `locales/zh.json` and `locales/en.json`, respectively) and do not go through the translation
  workflow.

## 4. Responsibilities of Directory Index Pages

Each top-level directory's `index.md` answers two questions: “What does this directory include?” and
“What does it exclude?” The index page points to this rule rather than maintaining a separate
content list—any handwritten list will inevitably become outdated (the sole authority for RFC status
is `rfc/TRACKING.md`, which is generated by a script; the same principle applies here).

## 5. Authority Chain and Prerequisites for Changes

**The authority chain is one-way**: RFC (establishes the basis) → SPEC / specification / design
documents. An SPEC may state, “See RFC-xxx for design rationale”; **an RFC must not use an SPEC as
its basis**—once an RFC depends on an SPEC, the SPEC can no longer be changed, and design authority
is undermined.

**Before changing documentation, you must read three things**:

1. **Upstream design**: Identify the RFC and the specific decision on which the conclusions of the
   document being changed are based (the decision registry D1–D59 in
   [RFC-039](../rfc/accepted/039-compiler-architecture.md), or a topic-specific RFC for that area).
   If you cannot identify one, this change has no basis.
2. **Adjacent documentation at the same level**: Check whether the same table or group of terms is
   already defined elsewhere—writing two versions is a repetition of §0, “Do not invent.”
3. **The red lines in [commit-convention.md](./commit-convention.md) and
   [coding-rules.md](./coding-rules.md).**

**Make corrections in place**: When documentation conflicts with the facts, change the body text
directly to match the facts and delete the old wording; do not retain comparisons such as “formerly
X, now Y” or “erratum”—the history is in git, and readers should not have to read the incorrect
version before the correct one.

**Criterion**: Every added or modified sentence must be able to answer “Who, under what conditions,
must do what?” Text that cannot answer this is explanatory and belongs in the RFC's “Motivation”
section, not in normative body text.
