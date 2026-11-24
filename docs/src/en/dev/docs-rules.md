---
title: 'Documentation Rules (docs-rules)'
description:
  'YaoXiang documentation classification rules (Diátaxis) and wiki-style writing guidelines'
---

# Documentation Rules (docs-rules)

> This article is the **canonical source** for documentation rules; before starting any work, use
> [HOWTO.md](./HOWTO.md) as the self-check entry point (the unified manual; Section 2 covers
> document changes). This article is at the same level as [coding-rules.md](./coding-rules.md):
> coding-rules governs code, while this article governs documentation.

## 0. Three Prohibitions (Documentation Edition)

These mirror the three prohibitions in coding-rules; reviews reject violations on the same basis:

1. **Do not invent**: Before introducing a new term or documentation type, first demonstrate that it
   does not duplicate an existing concept—check
   [docs/glossary.json](https://github.com/ChenXu233/yaoxiang/blob/main/docs/glossary.json), the
   classification table in §1, and grep existing documentation.
2. **Do not accumulate responsibilities**: A document belongs to only one quadrant and serves only
   one audience. Putting a reference table in a tutorial or design rationale in a reference document
   accumulates responsibilities.
3. **Refactor; do not patch**: If the same content appears in ≥2 places, one directory mixes two
   audiences, or one change touches ≥3 synonymous formulations—stop, adjust the structure first,
   then write the content.

## 1. Classification Rules: Six Top-Level Directories, Six Audiences

The documentation site organizes user documentation according to the four
[Diátaxis](https://diataxis.fr/) quadrants, plus two directories aimed at the repository itself:

| Directory      | Quadrant                             | Audience           | Reader's question                                            | Document form                 |
| -------------- | ------------------------------------ | ------------------ | ------------------------------------------------------------ | ----------------------------- |
| `tutorial/`    | Tutorial (learning-oriented)         | Beginners          | “Take me through building my first thing”                    | A progressive course          |
| `guide/`       | Guide (task-oriented)                | Users              | “I want to complete X; how do I do that?”                    | A step-by-step checklist      |
| `reference/`   | Reference (information-oriented)     | Users              | “What exactly does X mean?”                                  | A lookup-oriented description |
| `explanation/` | Explanation (understanding-oriented) | Curious readers    | “Why is it designed this way?”                               | An exposition                 |
| `dev/`         | — (Outside Diátaxis)                 | Contributors       | “I want to modify this repository; which rules do I follow?” | Rules, architecture, workflow |
| `rfc/`         | — (Language governance)              | Language designers | “How is this feature decided, and how do I propose one?”     | Proposals + status directory  |

`blog/` is a time-based, external narrative, and `archive/` is unpublished cold storage; neither
participates in classification.

### Decision Tree

1. Is the reader a **user** of the language or tool?
   - Yes → They want to **learn** (tutorial), **accomplish a task** (guide), **look something up**
     (reference), or **understand** (explanation)
   - No → 2
2. Does the reader need to **modify the compiler or repository**? → `dev/`
3. Is the reader **reviewing or submitting a language feature**? → `rfc/`

If a document cannot yield a single answer, it is serving two audiences—split it and place each part
where it belongs.

### Commonly Confused Boundaries (Empirical Cases)

| Content                                                                                    | Destination                   | Common misplacement                                           |
| ------------------------------------------------------------------------------------------ | ----------------------------- | ------------------------------------------------------------- |
| Command-line usage of `yx check`                                                           | `reference/check-command.md`  | dev/design/check/                                             |
| Design specification for `yx check` (zero-false-positive principle, incremental mechanism) | `dev/design/check/`           | reference/                                                    |
| Formatting rules (implementation behavior specifications)                                  | `dev/design/formatter/`       | reference/                                                    |
| Command-line usage of `yx format`                                                          | `reference/format-command.md` | dev/design/formatter/                                         |
| Language philosophy and manifestos (evergreen discourse)                                   | `explanation/`                | blog/ (blog is time-based)                                    |
| Compiler architecture and implementation roadmap                                           | `dev/architecture/`           | explanation/ (it is an operational manual, not an exposition) |
| Feature proposals and finalized decision records                                           | `rfc/`                        | dev/ (the audience for an RFC extends beyond contributors)    |

## 2. Language Style (Wiki-Style Writing)

This section draws on the core principles of Wikipedia's formatting manual and applies to all
directories. Adjust person and tone slightly by quadrant (§2.4).

### 2.1 Prefer the Active Voice

- Write “The compiler reports E1001,” not “E1001 is reported by the compiler.”
- Use the passive voice only when the actor is unknown or unimportant.
- Quick check: when a sentence contains a passive construction, ask “Who does it?” If there is an
  answer, rewrite it in the active voice.

### 2.2 Definite Statements

- Write “yx check does not generate code,” not “yx check may not generate code.”
- State facts definitively. There are only two valid ways to express uncertainty:
  - State a precise condition: “When `--watch` is enabled, yx check rechecks only affected files.”
  - Remove it, verify it, and then write it.
- Ban vague terms such as “may,” “perhaps,” “probably,” “basically,” and “to some extent.” Use
  “should” only to express obligation (“You should run the tests”), never speculation.

### 2.3 Present Tense

- Write “The parser rejects empty modules,” not “The parser will reject empty modules.”
  Documentation describes the behavior of the current version.
- Exception: when `rfc/draft/` and `rfc/review/` describe proposals that have not yet been
  implemented, use “will”; once the proposal has been implemented, return to the present tense.

### 2.4 Person

- `reference/`, `explanation/`, `rfc/`: use impersonal statements (“The function returns Result”).
- `tutorial/`, `guide/`: use the second person, “you” (“You run yx new”).
- `dev/`: rule statements should be impersonal, while operational instructions may use “you.”
- Do not use “we” in any directory: write “Recommended” instead of “We recommend.”

### 2.5 One Idea per Sentence

- A sentence should assert only one thing; if it contains two assertions, write two sentences.
- Use a list instead of a long sentence for three or more parallel facts.

### 2.6 Consistent Terminology

- A concept must have only one name across the entire site. If earlier text says “ownership,” later
  text must not switch to “proprietorship.”
- Before introducing a new term, check docs/glossary.json and existing usage in the documentation.
- On its first occurrence, define a term or link to its definition.

### 2.7 Objective Neutrality

- Do not write “very simple,” “only needs,” or “effortless.” When readers get stuck, such words read
  as mockery.
- Do not use marketing terms such as “powerful,” “elegant,” or “seamless.” Replace them with
  verifiable facts: “Compile-time monomorphization, with no runtime overhead.”
- When comparing other languages, state behavioral differences without evaluating which is better.

### 2.8 Verifiability

- Every behavioral claim must be substantiated by code or tests; do not write claims that cannot be
  substantiated.
- Code examples must be executable: CI (`scripts/ci/check-docs-examples.py`) runs them. Mark any
  non-executable example with `<!-- docs-example: skip -->` and provide a reason.

## 3. Structural and Mechanical Rules

- **Frontmatter**: `title` is required (sidebar generation depends on it), and `description` is
  recommended.
- **Headings**: Use one H1 per document, start the body at H2, and do not skip heading levels.
- **Links**: Internal links must always use a relative path to the target file, never a
  site-absolute path. The build configuration uses `ignoreDeadLinks: false`; dead links cause the
  build to fail.
- **Do not include line numbers**: Line numbers drift with every change, while `check-docs-refs.py`
  validates only whether a path resolves. Locate content through a semantic description (“the arm in
  `ir_gen` that obtains the span of `Expr::Cast`”); when a line number is genuinely necessary,
  retrieve it again from the referenced revision (`git show <rev>:<file>`).
- **Moving a file = changing the URL**: Synchronize all site-wide references, both sidebar
  configurations, the en mirror, scripts that reference the path, and AGENTS.md. For the complete
  process, see Section 2, “Six Synchronizations for Moving a Document,” in [HOWTO.md](./HOWTO.md).
- **Formatting**: Prettier (`docs/.prettierrc.json`) and markdownlint enforce table alignment and
  blank lines around fenced blocks.
- **Chinese/English mirrors**: zh is the source, and en is synchronized through the translation
  workflow. After adding or moving a zh file, its path on the en side must match.

## 4. Responsibilities of Directory Index Pages

The `index.md` for each top-level directory answers two questions: “What does this directory
include, and what does it exclude?” The index page points to these rules and does not separately
maintain a content list—any hand-written list will inevitably become outdated (the sole authority
for RFC status is `rfc/TRACKING.md`, which is generated by a script; the same principle applies).

## 5. Authority Chain and Prerequisites for Modification

**The authority chain is unidirectional**: RFC (establishes the basis) → SPEC / specifications /
design documents. An SPEC may state, “For design rationale, see RFC-xxx”; an **RFC must never use an
SPEC as its basis**—once an RFC depends on an SPEC, the SPEC can no longer be changed, effectively
undermining design authority.

**Before modifying a document, you must read three things**:

1. **Upstream design**: Which decision in which RFC the conclusion of the document being changed
   comes from (the D1–D59 decision register in
   [RFC-039](../rfc/accepted/039-compiler-architecture.md), or a subject-specific RFC in that area).
   If you cannot identify it, the change has no basis.
2. **Adjacent peer documents**: Whether the same table or set of terms has already been defined
   elsewhere—writing the same definition in two places is a variant of §0, “Do not invent.”
3. **The red lines in** [commit-convention.md](./commit-convention.md) **and**
   [coding-rules.md](./coding-rules.md).

**Correct in place**: When a document does not match the facts, change the body text directly to
match the facts and delete the old wording. Do not retain side-by-side wording such as “formerly...
now...” or “errata”—the history is in git, and readers should not have to read the incorrect version
before the correct one.

**Criterion**: Every sentence added or modified must be able to answer “who, under what conditions,
must do what.” A sentence that cannot is explanatory prose and belongs in the RFC's “Motivation”
section, not in the normative body.
