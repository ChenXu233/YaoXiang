---
title: 'Documentation Rules (docs-rules)'
description:
  'Classification rules (Diátaxis) and wiki-style writing conventions for YaoXiang documentation'
---

# Documentation Rules (docs-rules)

> This article is the authoritative definition of the documentation rules. Before starting work, use
> [HOWTO.md](./HOWTO.md) (the unified manual, whose second section covers documentation changes) as
> the self-check entry point. This article is a sibling to [coding-rules.md](./coding-rules.md):
> coding-rules governs code; this article governs documentation.

## 0. Three Prohibitions (Documentation Edition)

These mirror the three prohibitions in coding-rules; reviews must reject changes for the same
reasons:

1. **Do not coin new terms or document types**: Before introducing a new term or document type,
   first demonstrate that it does not duplicate an existing concept—check
   [docs/glossary.json](https://github.com/ChenXu233/yaoxiang/blob/main/docs/glossary.json), consult
   the §1 classification table, and grep the existing documentation.
2. **Do not accumulate responsibilities**: Each document belongs to only one quadrant and serves
   only one audience. Packing reference tables into a tutorial or design motivations into a
   reference is responsibility accumulation.
3. **Refactor, do not patch**: When the same content is duplicated in ≥2 places, a directory mixes
   two audiences, or a single change touches ≥3 synonymous formulations—stop, adjust the structure
   first, and then write the content.

## 1. Classification Rules: Six Top-Level Directories, Six Audiences

The documentation site organizes user documentation according to the four quadrants of
[Diátaxis](https://diataxis.fr/), with two additional directories for the repository itself:

| Directory      | Quadrant                             | Audience           | Reader's question                                             | Document form                  |
| -------------- | ------------------------------------ | ------------------ | ------------------------------------------------------------- | ------------------------------ |
| `tutorial/`    | Tutorial (learning-oriented)         | Beginners          | “Take me through building my first thing”                     | Progressive course             |
| `guide/`       | Guide (task-oriented)                | Users              | “I want to complete X; how do I do that?”                     | Checklist                      |
| `reference/`   | Reference (information-oriented)     | Users              | “What exactly does X mean?”                                   | Lookup-oriented description    |
| `explanation/` | Explanation (understanding-oriented) | Curious readers    | “Why was it designed this way?”                               | Exposition                     |
| `dev/`         | — (outside Diátaxis)                 | Contributors       | “I want to modify this repository; what rules must I follow?” | Rules, architecture, workflows |
| `rfc/`         | — (language governance)              | Language designers | “How is this feature decided on and proposed?”                | Proposals + status directory   |

`blog/` is a temporal external narrative, and `archive/` is a cold store excluded from publication;
neither participates in classification.

### Classification Decision Tree

1. Is the reader a **user** of the language or tool?
   - Yes → They want to **learn** (tutorial), **complete a task** (guide), **verify a fact**
     (reference), or **understand** (explanation)
   - No → 2
2. Does the reader need to **modify the compiler or repository**? → `dev/`
3. Is the reader **reviewing or submitting a language feature**? → `rfc/`

If a document does not lead to a unique answer, it is serving two audiences—split it and place each
part where it belongs.

### Ambiguous Boundaries (Empirical)

| Content                                                                                     | Destination                   | Common misplacement                                           |
| ------------------------------------------------------------------------------------------- | ----------------------------- | ------------------------------------------------------------- |
| Command-line usage of `yx check`                                                            | `reference/check-command.md`  | dev/design/check/                                             |
| Design specifications for `yx check` (zero-false-positive principle, incremental mechanism) | `dev/design/check/`           | reference/                                                    |
| Formatting-rule provisions (specified implementation behavior)                              | `dev/design/formatter/`       | reference/                                                    |
| Command-line usage of `yx format`                                                           | `reference/format-command.md` | dev/design/formatter/                                         |
| Language philosophy and manifestos (evergreen expositions)                                  | `explanation/`                | blog/ (blog is temporal)                                      |
| Compiler architecture and implementation roadmap                                            | `dev/architecture/`           | explanation/ (it is an operational manual, not an exposition) |
| Feature proposals and decision records                                                      | `rfc/`                        | dev/ (the RFC audience extends beyond contributors)           |

## 2. Writing Style (Wiki-Style)

This section draws on the core principles of Wikipedia's Manual of Style and applies to all
directories. Adjust pronouns and tone slightly by quadrant (§2.4).

### 2.1 Prefer the Active Voice

- Write “The compiler reports E1001,” not “E1001 is reported by the compiler.”
- Use the passive voice only when the actor is unknown or unimportant.
- Test: whenever a sentence contains a passive construction such as “is reported by,” “is generated
  by,” or “is achieved through,” ask once, “Who did it?” If there is an answer, rewrite the sentence
  in the active voice.

### 2.2 Use Definite Statements

- Write “yx check does not generate code,” not “yx check may not generate code.”
- Facts must be asserted. There are only two valid ways to express uncertain content:
  - State the precise condition: “When `--watch` is enabled, yx check rechecks only affected files.”
  - Remove it, verify it, and then write it.
- Avoid vague words such as may, might, perhaps, probably, basically, or to some extent. Use
  “should” only to express an obligation (“You should run the tests”), never speculation.

### 2.3 Use the Present Tense

- Write “The parser rejects empty modules,” not “The parser will reject empty modules.”
  Documentation describes the behavior of the current version.
- Exception: `rfc/draft/` and `rfc/review/` use “will” when describing proposals that have not yet
  been implemented; switch back to the present tense after implementation.

### 2.4 Point of View

- `reference/`, `explanation/`, `rfc/`: Use impersonal statements (“The function returns Result”).
- `tutorial/`, `guide/`: Use the second person, “you” (“You run yx new”).
- `dev/`: Write rules impersonally; operational instructions may use “you.”
- No directory may use “we”: Replace “We recommend” with “Recommended.”

### 2.5 One Idea per Sentence

- A sentence makes only one assertion; use two sentences for two assertions.
- Use a list instead of a long sentence for three or more parallel facts.

### 2.6 Consistent Terminology

- A concept must have only one name throughout the site. Use “ownership” consistently; do not later
  switch to “proprietary rights.”
- Before introducing a new term, check docs/glossary.json and its use in the existing documentation.
- Define a term on its first appearance, or link to its definition.

### 2.7 Objectivity and Neutrality

- Do not write “very simple,” “only need to,” or “effortless.” When readers get stuck, such words
  are taunts.
- Do not use marketing terms such as powerful, elegant, or seamless. Replace them with verifiable
  facts: “Compile-time monomorphization, with no runtime overhead.”
- When comparing other languages, state behavioral differences without judging which is better.

### 2.8 Verifiability

- Every behavioral assertion must be verifiable by code or tests; do not make assertions that cannot
  be verified.
- Code examples must be executable: CI (`scripts/ci/check-docs-examples.py`) runs them. Mark
  examples that cannot be executed with `<!-- docs-example: skip -->` and provide a reason.

## 3. Structural and Mechanical Rules

- **Frontmatter**: `title` is required (sidebar generation depends on it); `description` is
  recommended.
- **Headings**: Each document has one H1. The body starts at H2, with no skipped levels.
- **Links**: All internal links must use a relative path to the target file, not a site-absolute
  path. The build configuration `ignoreDeadLinks: false` means that dead links cause the build to
  fail.
- **Do not include line numbers**: Line numbers drift with every change, while `check-docs-refs.py`
  checks only whether the path resolves. Locate content with a semantic description (“the arm in
  `ir_gen` that retrieves the span of `Expr::Cast`”); if a line number is genuinely required,
  retrieve it again from the referenced revision (`git show <rev>:<file>`). **Exception**:
  Build-generated error-code reference pages include a `file:line` emission-point anchor—the
  generator calculates the line number from the current source at build time, so it is always
  current.
- **Moving a file = changing the URL**: All site-wide references, both sidebar configurations, the
  en/ mirror, scripts that reference the path, and AGENTS.md must be updated in sync. For the
  complete procedure, see Section 2, “Six Synchronizations for Moving Documentation,” in
  [HOWTO.md](./HOWTO.md).
- **Formatting**: Table alignment and blank lines in fenced code blocks are enforced by prettier
  (`docs/.prettierrc.json`) and markdownlint.
- **Chinese-English mirror**: zh is the source, and en is synchronized through the translation
  workflow. After adding or moving a zh file, its path must match on the en side. **Exception**:
  Error-code reference pages are not checked in; both the zh and en versions are generated at build
  time by `scripts/docs/gen-error-code-docs.py` (with text taken from `locales/zh.json` and
  `locales/en.json`, respectively) and do not pass through the translation workflow.

## 4. Responsibilities of Directory Index Pages

Each top-level directory's `index.md` answers two questions: “What does this directory include, and
what does it exclude?” The index page links to this rule rather than duplicating a content list that
it must maintain by hand; such a list inevitably becomes stale. The sole authority for RFC status is
`rfc/TRACKING.md`, which is generated by a script; the same principle applies to other generated
listings.

## 5. Authority Chain and Prerequisites for Changes

**The authority chain is unidirectional**: RFC (establishes the rationale) → SPEC / specifications /
design documents. A SPEC may state, “See RFC-xxx for the design rationale”; an **RFC must not use a
SPEC as its basis**—once it does, the SPEC can no longer be changed, undermining the authority to
make design decisions.

**Before modifying documentation, read the following three items**:

1. **Upstream design**: Which decision in which RFC provides the basis for the conclusions in the
   document being changed (the decision registry D1–D64 in
   [RFC-039](../rfc/accepted/039-compiler-architecture.md), or a domain-specific RFC). If you cannot
   specify it, the change has no basis.
2. **Adjacent peer documents**: Whether the same table or set of terms is already defined
   elsewhere—writing separate copies in two places is a variant of §0's “Do not coin new terms.”
3. The red lines in [commit-convention.md](./commit-convention.md) and
   [coding-rules.md](./coding-rules.md).

**Make corrections in place**: When a document does not match the facts, change the body text to
match the facts and delete the old wording; do not retain before-and-after comparisons such as
“formerly … now …” or “erratum.” History is in git, and readers should not have to read the wrong
version before the correct one.

**Criterion**: Every added or modified sentence must answer “who, under what conditions, must do
what.” A sentence that cannot answer this is explanatory prose and belongs in the RFC's “Motivation”
section, not in normative body text.
