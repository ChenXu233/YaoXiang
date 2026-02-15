---
title: RFC Index
---


# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for YaoXiang language feature design proposals.

## Table of Contents

- [YaoXiang RFC (Request for Comments) Index](#yaoxiang-rfc-request-for-comments-index)
  - [Table of Contents](#table-of-contents)
  - [Templates](#templates)
  - [Draft RFCs](#draft-rfcs)
  - [RFCs Under Review](#rfcs-under-review)
  - [Accepted RFCs](#accepted-rfcs)
  - [Rejected RFCs](#rejected-rfcs)
  - [RFC Lifecycle](#rfc-lifecycle)
    - [Status Description](#status-description)
  - [Submitting an RFC](#submitting-an-rfc)
  - [Contribution Guide](#contribution-guide)

---

## Templates

| File | Description |
|------|-------------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC standard template |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete example (pattern matching enhancement) |

---

## Draft RFCs

| ID | Title | Author | Created Date | Status |
|----|-------|--------|--------------|--------|
| RFC-002 | [Cross-platform I/O with libuv Integration](./draft/002-cross-platform-io-libuv.md) | ChenXu | 2025-01-05 | Draft |
| RFC-005 | [Automated CVE Security Scanning System](./draft/005-automated-cve-scanning.md) | ChenXu | 2025-01-05 | Draft |
| RFC-012 | [F-String Template Strings](./draft/012-f-string-template-strings.md) | ChenXu | 2025-01-27 | Draft |
| RFC-016 | [Quantum-Native Support and Multi-Backend Integration](./draft/016-quantum-native-support.md) | ChenXu | 2026-02-12 | Draft |
| RFC-017 | [Yaoxiang LSP Support](./draft/017-yaoxiang-lsp-support.md) | ChenXu | 2026-02-12 | Draft |

---

## RFCs Under Review

| ID | Title | Author | Created Date | Status |
|----|-------|--------|--------------|--------|
| RFC-003 | [Version Planning and Implementation Suggestions](./review/003-version-planning.md) | ChenXu | 2025-01-05 | Under Review |

---

## Accepted RFCs

| ID | Title | Author | Created Date | Status |
|----|-------|--------|--------------|--------|
| RFC-001 | [Concurrency Model and Error Handling System](./accepted/001-concurrent-model-error-handling.md) | ChenXu | 2025-01-05 | Accepted |
| RFC-004 | [Multi-Position Joint Binding Design for Curried Methods](./accepted/004-curry-multi-position-binding.md) | ChenXu | 2025-01-05 | Accepted |
| RFC-006 | [Documentation Site Construction and Optimization Plan](./accepted/006-documentation-site-optimization.md) | ChenXu | 2025-01-05 | Accepted |
| RFC-007 | [Function Definition Syntax Unification Scheme](./accepted/007-function-syntax-unification.md) | ChenXu | 2025-01-05 | Accepted |
| RFC-008 | [Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md) | ChenXu | 2025-01-05 | Accepted |
| RFC-009 | [Ownership Model v7](./accepted/009-ownership-model.md) | ChenXu | 2025-01-05 | Accepted |
| RFC-011 | [Generic Type System Design - Zero-Cost Abstraction and Macro Replacement](./accepted/011-generic-type-system.md) | ChenXu | 2025-01-25 | Accepted |
| RFC-013 | [Error Code Specification Design](./accepted/012-error-code-specification.md) | ChenXu | 2025-01-30 | Accepted |
| RFC-014 | [Package Manager System Design](./accepted/014-package-manager.md) | ChenXu | 2026-02-12 | Accepted |
| RFC-015 | [yaoxiang.toml Fields Research](./draft/015-yaoxiang-toml-fields.md) | ChenXu | 2026-02-12 | Accepted |

---

## Rejected RFCs

None yet

---

## RFC Lifecycle

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review │  ← Open for community discussion and feedback
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  accepted/  │    │  rejected/  │
│ (Final Design)|   │ (Rejected)  │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|-----------|-------------|
| **Draft** | `docs/reference/rfc/draft/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/reference/rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `docs/reference/rfc/accepted/` | Becomes formal design document, enters implementation phase |
| **Rejected** | `docs/reference/rfc/rejected/` | Rejected RFC documents |

---

## Submitting an RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand format requirements
2. Refer to [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the writing style
3. Create a new file, named `ID-descriptive-title.md`
4. Put the file in `docs/reference/rfc/draft/` directory
5. Update this index file, add new RFC entry
6. Submit PR to enter review process

---

## Contribution Guide

Please refer to [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for contribution guidelines.
