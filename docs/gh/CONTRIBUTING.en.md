# Contributing Guide

> Thank you for your interest in YaoXiang! We welcome contributions of all kinds.

> 🌐 **Language** | [中文](../../CONTRIBUTING.md)

---

## Table of Contents

- [Ways to Contribute](#ways-to-contribute)
- [Getting Started](#getting-started)
- [Submitting Changes](#submitting-changes)
- [RFC Process](#rfc-process)
- [Code Placement & Change Protocol](#code-placement--change-protocol)
- [Code Standards](#code-standards)
- [Documentation Checklist](#documentation-checklist)
- [Contributor License Agreement (CLA)](#contributor-license-agreement-cla)
- [Code Review](#code-review)
- [Community Resources](#community-resources)
- [Code of Conduct](#code-of-conduct)

---

## Ways to Contribute

| Ways |
| --- |
| Report bugs in GitHub Issues |
| Propose new features or designs |
| Write or improve documentation |
| Submit code fixes or new features |
| Language design, logo, UI |

---

## Getting Started

### Prerequisites

```bash
# Install Rust (recommended: rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# Build the project
cargo build --release

# Run tests
cargo test
```

### Code Style

```bash
# Format code
cargo fmt

# Type checking
cargo check

# Run all checks
cargo clippy
```

---

## Submitting Changes

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Commit Convention

Follow the [commit convention](../src/en/dev/commit-convention.md):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types**:

| Type | Meaning |
|------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `style` | Code formatting (no functional changes) |
| `refactor` | Code refactoring |
| `test` | Add or modify tests |
| `chore` | Build tool or auxiliary changes |

**Example**:

```
feat(frontend): Add type inference feature

Implemented basic polymorphic type inference algorithm.

Closes #123
```

### 3. Submit a PR

1. Push your branch: `git push origin feature/your-feature-name`
2. Visit GitHub to create a Pull Request
3. Fill in the PR template
4. Wait for code review

---

## RFC Process

### Submit an RFC

For new features or major changes, please submit an RFC first:

1. Read the [RFC Template](../src/en/rfc/RFC_TEMPLATE.md)
2. Reference the [Full Example](../src/en/rfc/EXAMPLE_full_feature_proposal.md)
3. Create a new RFC file in `docs/src/rfc/`
4. Set status to "Draft" or "Review"
5. Submit a PR for discussion

### RFC Lifecycle

```
Draft → Review → Accepted → accepted/
               → Rejected → stays in rfc/
```

See [RFC Template](../src/en/rfc/RFC_TEMPLATE.md) for details.

### RFC Implementation & Documentation Updates

> **Every accepted RFC's implementation PR must include documentation updates.**

- The implementation PR must check "affects documentation, I have updated related docs" in the PR template
- If the implementation PR contains no documentation updates, it must explicitly state "no documentation impact" in the PR description with a reason
- `scripts/rfc/check_tracking.py` automatically checks: RFC accepted status + whether the corresponding implementation PR contains documentation changes
- Documentation update scope includes: tutorial (if the RFC introduces a new feature), reference/language-spec (language specification), reference/error-code (new error codes), etc.

---

## Code Placement & Change Protocol

> This section is a **mandatory protocol** that applies to all code changes, especially during the compiler architecture refactor ([RFC-039](../src/en/rfc/accepted/039-compiler-architecture.md)).
> Before starting work, read the [implementer's manual HOWTO.md](../src/en/dev/HOWTO.md) (self-check list + patch determination); the rules themselves live in [coding-rules.md](../src/en/dev/coding-rules.md).

### Three Prohibitions

1. **No fabrication** — Before adding any `pub` type / enum / constant table, prove it does not duplicate an existing concept (criteria A–D in coding-rules: responsibility overlap, insufficient call sites, disambiguation alias required, synonym-table bridging; any hit is a violation)
2. **No responsibility accumulation** — One module carries one kind of responsibility. To add a new responsibility, create a new module or move the existing code there. No line-count gates; responsibility judgment is human (which is exactly why machines cannot replace it)
3. **Refactor, don't patch** — Stop and go through the design-doc process when any of these hits: the same behavior must be duplicated in ≥2 places; adding an "N-th entry point" instead of registering into the declarative stage table; one change requires syncing ≥3 synonymous mappings

### Decision Procedure (D0–D4)

Every change passes through five gates in order (each is a decidable boolean; stop at the first hit; full text in coding-rules part 2):

| Gate | Question | On hit |
| --- | --- | --- |
| D0 | Does the change touch any existing table (error codes / opcodes / types / stages)? Which module is the single authoritative source? | Cannot name the authority → ⛔ establish the authority first |
| D1 | Can the new concept be expressed with "existing concepts + parameters"? | Yes → ⛔ do not add it |
| D2 | Same semantics as an existing concept (variant-name overlap ≥ half)? | No From/TryFrom → ⛔ merge into one; no import-alias bridging |
| D3 | Duplication in ≥2 places / new entry point / ≥3 synonymous mappings? | Hit → ⛔ stop for the design process; otherwise → local patch allowed + regression test |
| D4 | Does the new code belong to the target module's **existing responsibility categories**? (responsibility table in [01-routing.md](../src/en/dev/architecture/01-routing.md)) | Second category or beyond → ⛔ create a new module |

### Hard Rules (review rejects on sight)

- Never change code you have not read: do not touch it without opening the file and grepping the reference sites
- No `todo!()` / "Not implemented yet" / indefinite "separate issue" left in core functionality
- No deleting tests or relaxing criteria for green (there is no C5′; being unable to meet a criterion is an implementation defect — report it honestly)
- The PR template's "Responsibility & Decision Procedure" block is **required**; being unable to name the D0 authority module means you did not check

---

## Code Standards

### Rust Code

- Follow `rustfmt` default formatting
- Use `clippy` for linting
- Add appropriate comments and documentation
- Write rustdoc for public APIs

### Documentation

- Use consistent terminology
- Mark code blocks with their language
- Follow [documentation rules](../src/en/dev/docs-rules.md)

### Tests

- Add unit tests for new features
- Update integration tests if needed
- Ensure all tests pass

---

## Documentation Checklist

> Every PR must ask itself one question before submission: **"Does this change require documentation updates?"**
>
> The criteria below help you decide quickly. The AI reviewer automatically checks the PR's documentation impact assessment.

### Documentation update required

| Case | Description |
|:----|:-----|
| Public API added/changed | Function signatures, type definitions, traits, or other public interfaces changed |
| CLI behavior changed | Command-line arguments, subcommands, or output formats added/removed/modified |
| Config format or defaults changed | Fields in `yaoxiang.toml` or other config files changed |
| Feature added/removed | Language features, compiler functionality, or toolchain functionality changed |
| Bug fix for doc/behavior inconsistency | Documentation did not match actual behavior; update docs alongside the fix |
| Error or warning code added | Must update `docs/src/reference/error-code/` or `warning-code/` |

### Documentation update not required

| Case | Description |
|:----|:-----|
| Pure bugfix (no behavior description change) | Fixed internal logic errors; no user-visible behavior change |
| Pure refactoring (no external behavior change) | Code reorganization or performance optimization; public interfaces unchanged |
| Pure test additions | New test cases or test fixes; no documented functionality involved |
| Documentation-only update | Directly modifies documentation content; no extra explanation needed |
| CI/build toolchain change | CI configuration, build scripts, or dependency versions |

### Self-check flow

```
My PR changes code
  ├─ Does it change user-visible behavior?
  │   ├─ Yes → documentation update required → check "updated" in the PR template
  │   └─ No → next step
  ├─ Does it add a public API?
  │   ├─ Yes → documentation update required → check "updated" in the PR template
  │   └─ No → next step
  └─ None of the above → check "no documentation impact" in the PR template
```

> **Note**: Choosing "affects documentation but not updated" will be blocked by CI and require a justification.

---

## Contributor License Agreement (CLA)

Before your first Pull Request can be merged, you must sign our [Contributor License Agreement (CLA)](../../CLA.md).

The CLA Assistant bot will automatically comment on your PR with instructions.

**What the CLA covers**:

| Coverage |
| --- |
| Copyright license to use your contributions |
| Patent license for any patents your contributions may cover |
| Confirmation that your work is original and does not infringe third-party rights |

---

## Code Review

### Review Checklist

| Item |
| --- |
| Code is functionally correct |
| Follows code standards |
| Has appropriate tests |
| Documentation is updated (see [Documentation Checklist](#documentation-checklist)) |
| No performance regression |

### Responding to Feedback

| Practice |
| --- |
| Reply to review comments in a timely manner |
| Explain your design decisions |
| Be open to reasonable suggestions |

---

## Community Resources

- GitHub Issues: report bugs
- GitHub Discussions: community chat
- Project documentation

---

## Code of Conduct

This project follows our [Code of Conduct](../../CODE_OF_CONDUCT.md).
Please be respectful and friendly when contributing to our community.

---

> Thank you for your contribution!
>
> For questions, feel free to discuss in GitHub Discussions.
