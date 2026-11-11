---
title: 'Documentation Rules (docs-rules)'
description:
  'Classification rules (Diátaxis) and wiki-style language standards for YaoXiang documentation'
---

# Documentation Rules (docs-rules)

> This document is the **authoritative source** for documentation rules; before making changes, use
> [HOWTO.md](./HOWTO.md) (the unified manual; Section 2 covers modifying documentation) as the entry
> point for pre-change self-checks. This document is at the same level as
> [coding-rules.md](./coding-rules.md): coding-rules governs code, while this document governs
> documentation.

## 0. Three Prohibitions (Documentation Edition)

These mirror the three prohibitions in coding-rules; reviews should reject changes on the same
grounds:

1. **Do not invent terminology or document types**: Before introducing a new term or document type,
   first prove that it does not duplicate an existing concept—check
   [docs/glossary.json](https://github.com/ChenXu233/yaoxiang/blob/main/docs/glossary.json), consult
   the §1 classification table, and grep the existing documentation.
2. **Do not accumulate responsibilities**: A document belongs to only one quadrant and serves only
   one audience. Putting a reference table in a tutorial or design rationale in a reference is an
   accumulation of responsibilities.
3. **Refactor rather than patch**: If the same content appears in ≥2 places, a directory mixes two
   audiences, or a single change affects ≥3 synonymous statements—stop, adjust the structure first,
   and then write the content.

## 1. Classification Rules: Six Top-Level Directories, Six Audiences

The documentation site organizes user documentation according to the four
[Diátaxis](https://diataxis.fr/) quadrants, plus two directories for the repository itself:

| Directory      | Quadrant                             | Audience                | Reader’s question                                             | Document form                      |
| -------------- | ------------------------------------ | ----------------------- | ------------------------------------------------------------- | ---------------------------------- |
| `tutorial/`    | Tutorial (learning-oriented)         | Beginners               | “Take me through making my first thing”                       | Progressive course                 |
| `guide/`       | Guide (task-oriented)                | Users                   | “I need to complete X; how do I do it?”                       | Step-by-step checklist             |
| `reference/`   | Reference (information-oriented)     | Users                   | “What does X mean exactly?”                                   | Lookup-oriented description        |
| `explanation/` | Explanation (understanding-oriented) | Readers who are curious | “Why is it designed this way?”                                | Expository discussion              |
| `dev/`         | — (outside Diátaxis)                 | Contributors            | “I want to modify this repository; what rules must I follow?” | Rules, architecture, and processes |
| `rfc/`         | — (language governance)              | Language designers      | “How is this feature decided, and how do I propose one?”      | Proposals + status directory       |

`blog/` is for time-bound, outward-facing narratives, while `archive/` is cold storage excluded from
publication; neither participates in the classification.

### Decision Tree

1. Is the reader a **user** of the language or tool?
   - Yes → They want to **learn** (tutorial), **accomplish a task** (guide), **look something up**
     (reference), or **understand** (explanation)
   - No → 2
2. Does the reader need to **modify the compiler or repository**? → `dev/`
3. Is the reader **reviewing or submitting a language feature**? → `rfc/`

If a document cannot yield a unique answer, it is serving two audiences—split it and put each part
in its proper place.

### Commonly Confused Boundaries (Empirical Examples)

| Content                                                                                    | Destination                   | Common incorrect destination                                |
| ------------------------------------------------------------------------------------------ | ----------------------------- | ----------------------------------------------------------- |
| Command-line usage of `yx check`                                                           | `reference/check-command.md`  | `dev/design/check/`                                         |
| Design specification for `yx check` (zero-false-positive principle, incremental mechanism) | `dev/design/check/`           | `reference/`                                                |
| Formatting rule clauses (implementation behavior specifications)                           | `dev/design/formatter/`       | `reference/`                                                |
| Command-line usage of `yx format`                                                          | `reference/format-command.md` | `dev/design/formatter/`                                     |
| Language philosophy and manifesto (evergreen discourse)                                    | `explanation/`                | `blog/` (blog is time-bound)                                |
| Compiler architecture and implementation plan                                              | `dev/architecture/`           | `explanation/` (it is an operations manual, not exposition) |
| Feature proposals and decision records                                                     | `rfc/`                        | `dev/` (the RFC audience extends beyond contributors)       |

## 2. Language Style (Wiki-Style Writing)

This section follows the core principles of Wikipedia’s formatting manual and applies to all
directories; person and tone vary slightly by quadrant (§2.4).

### 2.1 Prefer the Active Voice

- Write “The compiler reports E1001,” not “E1001 is reported by the compiler.”
- Use the passive voice only when the actor is unknown or unimportant.
- Test: Whenever a sentence uses a passive construction, ask “Who did it?” If there is an answer,
  rewrite the sentence in the active voice.

### 2.2 Make Definite Assertions

- Write “yx check does not generate code,” not “yx check may not generate code.”
- State facts as assertions. Uncertain content has only two acceptable forms:
  - State a precise condition: “When `--watch` is enabled, yx check rechecks only the affected
    files.”
  - Delete it, then write it after verification.
- Do not use vague wording: “may,” “perhaps,” “probably,” “basically,” or “to some extent.” Use
  “should” only to express obligation (“You should run the tests”), never speculation.

### 2.3 Use the Present Tense

- Write “The parser rejects empty modules,” not “The parser will reject empty modules.”
  Documentation describes the behavior of the current version.
- Exception: When `rfc/draft/` and `rfc/review/` describe proposals that have not yet been
  implemented, use “will”; after the proposal is implemented, return to the present tense.

### 2.4 Person

- `reference/`, `explanation/`, `rfc/`: Use impersonal statements (“The function returns Result”).
- `tutorial/`, `guide/`: Use the second person, “you” (“You run yx new”).
- `dev/`: Keep rule statements impersonal; operational instructions may use “you.”
- Never use “we” in any directory: Write “Recommended,” not “We recommend.”

### 2.5 One Idea per Sentence

- A sentence asserts one thing; if it contains two assertions, write two sentences.
- Use a list for three or more parallel facts instead of a long sentence.

### 2.6 Consistent Terminology

- A concept has only one name across the entire site. If an earlier passage says “ownership,” a
  later passage must not switch to “proprietorship.”
- Before adding a term, check `docs/glossary.json` and its usage in the existing documentation.
- At its first occurrence, provide a definition or link to its definition.

### 2.7 Objective Neutrality

- Do not write “very simple,” “only need to,” or “effortless.” When readers get stuck, such words
  mock them.
- Do not use marketing terms such as “powerful,” “elegant,” or “seamless.” Replace them with
  verifiable facts: “Compile-time monomorphization, with no runtime overhead.”
- When comparing other languages, state behavioral differences rather than evaluating merits.

### 2.8 Verifiability

- Every behavioral assertion must be verifiable through code or tests; do not write assertions that
  cannot be substantiated.
- Code examples must be executable: CI (`scripts/ci/check-docs-examples.py`) runs them. Mark
  examples that cannot be executed with `<!-- docs-example: skip -->` and provide a reason.

## 3. Structural and Mechanical Rules

- **Frontmatter**: `title` is required (sidebar generation depends on it); `description` is
  recommended.
- **Headings**: Each document has one H1, the body starts at H2, and heading levels are not skipped.
- **Links**: All internal links must use relative paths to the target files, not absolute site
  paths. The build configuration `ignoreDeadLinks: false` causes dead links to fail the build.
- **Moving a file = changing the URL**: All site references, both sidebar configurations, the `en/`
  mirror, scripts that reference the path, and AGENTS.md must be updated in sync. See Section 2,
  “Six Synchronization Steps for Moving Documents,” of [HOWTO.md](./HOWTO.md) for the complete
  process.
- **Formatting**: Prettier (`docs/.prettierrc.json`) and markdownlint enforce table alignment and
  blank lines in fenced code blocks.
- **Chinese–English mirror**: zh is the source, and en is synchronized by the translation workflow.
  After adding or moving a zh file, the path on the en side must match.

## 4. Responsibilities of Directory Index Pages

The `index.md` page in each top-level directory answers two questions: “What belongs in this
directory, and what does not?” The index page links to these rules and does not duplicate the
content list—a manually maintained list will inevitably become outdated (the sole source of truth
for RFC status is `rfc/TRACKING.md`, which is generated by a script; the same principle applies
here).

## 5. Authority Chain and Prerequisites for Modification

**The authority chain is unidirectional**: RFC (establishes the basis) → SPEC / specification /
design documentation. A SPEC may say “For design rationale, see RFC-xxx”; an **RFC must not use a
SPEC as its basis**—once an RFC depends on a SPEC, the SPEC can never be changed again, and design
authority is undermined.

**Before modifying documentation, you must read three things**:

1. **Upstream design**: Identify which decision in which RFC is the basis for the document being
   changed (the D1–D59 decision register in [RFC-039](../rfc/accepted/039-compiler-architecture.md),
   or a specialized RFC for that domain). If you cannot identify one, this change has no basis.
2. **Peer documents at the same level**: Check whether the same table or group of terms is already
   defined elsewhere—maintaining separate copies is just the “Do not invent terminology or document
   types” rule from §0 all over again.
3. The red lines in **[commit-convention.md](./commit-convention.md)** and
   [coding-rules.md](./coding-rules.md).

**Correct in place**: If documentation conflicts with the facts, directly change the body to match
the facts and delete the old wording; do not retain comparisons such as “Originally it was … now it
is …” or “Erratum”—history is in git, and readers should not have to read the wrong version before
the correct one.

**Criterion**: Every added or modified sentence must be able to answer “Who must do what under what
conditions?” A sentence that cannot is explanatory prose and belongs in the RFC’s “Motivation”
section, not in the normative body.
