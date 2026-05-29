> **⚠️ Note: This document is outdated and for reference only.**  
>
> The content described in this document is no longer applicable. Please refer to the latest documentation.

# YaoXiang Documentation Directory Reorganization Plan

## Overview

**Goal**: Reorganize the `docs/` directory to create a standalone **Design Discussion** and **Implementation Plan Tracking** directory

---

## Target Directory Structure

```
docs/
├── design/                    # ⭐ Design Discussion Area (new core directory)
│   ├── README.md              # Design document index
│   ├── manifesto.md           # Design manifesto
│   ├── language-spec.md       # Language specification
│   ├── async-whitepaper.md    # Async whitepaper
│   ├── 00-wtf.md              # Design tradeoffs (FAQ)
│   └── 01-philosophy.md       # Design philosophy (formerly "A 2006-born Person's Language Design Views.md")
│
├── design/rfc/                # RFC-style design proposals (optional)
│   └── (future proposals)
│
├── design/discussion/         # Open discussion area (drafts)
│   └── (design documents under discussion)
│
├── plans/                     # ⭐ Implementation plans (promoted from works/plans)
│   ├── README.md
│   ├── book-improvement.md
│   ├── stdlib-implementation.md
│   ├── test-organization.md
│   └── async/
│       ├── implementation-plan.md
│       └── threading-safety.md
│
├── implementation/            # ⭐ Implementation tracking (new)
│   ├── README.md
│   ├── phase1/
│   │   └── type-check-inference.md
│   └── phase5/
│       ├── bytecode-generation.md
│       └── gap-analysis.md
│
├── architecture/              # Architecture design (kept)
├── guides/                    # Usage guides (kept)
├── examples/                  # Example code (kept)
└── reference/                 # Reference documentation (kept)
```

---

## Directory Responsibilities

| Directory | Responsibility | Content Type |
|-----------|----------------|--------------|
| `design/` | Completed design decision discussions | Manifesto, specifications, whitepapers, design tradeoffs |
| `design/rfc/` | Designs in proposal stage (optional) | RFC documents, drafts |
| `design/discussion/` | Designs pending discussion | Open questions, drafts under discussion |
| `plans/` | Implementation plans intended | Implementation roadmap, task breakdown |
| `implementation/` | Completed/in-progress implementation details | Technical details, phase reports |

---

## Migration Checklist

### 1. Move to `design/`

| Original Location | New Location |
|-------------------|--------------|
| `docs/YaoXiang-design-manifesto.md` | `docs/design/manifesto.md` |
| `docs/YaoXiang-language-specification.md` | `docs/design/language-spec.md` |
| `docs/YaoXiang-async-whitepaper.md` | `docs/design/async-whitepaper.md` |
| `docs/YaoXiang-WTF.md` | `docs/design/00-wtf.md` |
| `docs/一个2006年出生者的语言设计观.md` | `docs/design/01-philosophy.md` |

### 2. Promote `works/plans/` to Root Level

| Original Location | New Location |
|-------------------|--------------|
| `docs/plans/` | `docs/plans/` |

### 3. Move to `implementation/`

| Original Location | New Location |
|-------------------|--------------|
| `docs/works/phase/phase1/type-check-inference-rules.md` | `docs/implementation/phase1/type-check-inference.md` |
| `docs/works/phase/phase5/phase5-bytecode-generation.md` | `docs/implementation/phase5/bytecode-generation.md` |
| `docs/works/phase/phase5/phase5-implementation-gap-analysis.md` | `docs/implementation/phase5/gap-analysis.md` |

### 4. Keep As-Is

| Directory | Notes |
|-----------|-------|
| `docs/architecture/` | Architecture design is already independent, keep as-is |
| `docs/guides/` | User guides are already independent, keep as-is |
| `docs/examples/` | Example code, keep as-is |
| `docs/works/old/` | Historical archive, keep or delete |
| `docs/plans/async/` | Already promoted to `plans/async/` |

### 5. Optional: Update `docs/README.md`

Need to update the documentation index to reflect the new directory structure.

---

## Execution Steps

### Step 1: Create Directory Structure

```bash
mkdir -p docs/design/discussion
mkdir -p docs/design/rfc
mkdir -p docs/plans/async
mkdir -p docs/implementation/phase1
mkdir -p docs/implementation/phase5
```

### Step 2: Move Design Documents

```bash
# Move to design/
mv docs/YaoXiang-design-manifesto.md docs/design/manifesto.md
mv docs/YaoXiang-language-specification.md docs/design/language-spec.md
mv docs/YaoXiang-async-whitepaper.md docs/design/async-whitepaper.md
mv docs/YaoXiang-WTF.md docs/design/00-wtf.md
mv "docs/一个2006年出生者的语言设计观.md" docs/design/01-philosophy.md

# Move to design/discussion/ (optional: for drafts pending discussion)
```

### Step 3: Promote Plans Directory

```bash
# Move works/plans to root level
mv docs/plans/* docs/plans/
rmdir docs/works/plans
```

### Step 4: Move Implementation Documents

```bash
# Move to implementation/
mv docs/works/phase/phase1/type-check-inference-rules.md docs/implementation/phase1/type-check-inference.md
mv docs/works/phase/phase5/phase5-bytecode-generation.md docs/implementation/phase5/bytecode-generation.md
mv docs/works/phase/phase5/phase5-implementation-gap-analysis.md docs/implementation/phase5/gap-analysis.md
```

### Step 5: Update docs/README.md

Update the documentation index, adding new directory descriptions.

### Step 6: Clean Up Empty Directories

```bash
rmdir docs/works/phase/phase5
rmdir docs/works/phase/phase1
rmdir docs/works/phase
rmdir docs/works/old/archived
rmdir docs/works/old
```

---

## Backward Compatibility

⚠️ **Important**: This reorganization will break existing references. Recommended actions:

1. **Do not delete original files**, create symlinks first or verify after moving
2. **Update all internal links**: Check relative path references in `docs/**/*.md`
3. **Update IDE configurations**: If `.vscode` or other configurations exist

---

## Expected Benefits

1. **Clear responsibilities**: Design vs. Plan vs. Implementation, boundaries are clear
2. **Easy access**: `design/` and `plans/` are at the root level, no need to dig into `works/`
3. **Scalability**: New `design/rfc/` and `design/discussion/` support RFC workflow
4. **Clear documentation types**: Completed designs, pending designs, implementation plans, and implementation tracking are all in their proper places

---

## Notes

- Confirm whether the archived content in `works/` needs to be kept
- Check if any other documents reference these file paths
- Consider whether an RFC template needs to be created for `design/rfc/`