---
title: RFC Index
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for YaoXiang language feature design proposals.

## Table of Contents

- [Template](#template)
- [Draft RFCs](#draft-rfcs)
- [RFCs Under Review](#rfcs-under-review)
- [Accepted RFCs](#accepted-rfcs)
- [Rejected RFCs](#rejected-rfcs)

---

## Template

| File | Description |
|------|-------------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC standard template |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete example (pattern matching enhancement) |

---

## Draft RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-016 | [Quantum-Native Support and Multi-Backend Integration](./draft/016-quantum-native-support.md) | Chenxu | 2026-02-12 | Draft |
| RFC-018 | [LLVM AOT Compiler and Runtime Scheduler Integration Design](./draft/018-llvm-aot-compiler.md) | Chenxu | 2026-02-15 | Draft |
| RFC-019 | [Typed Homoiconicity](./draft/019-typed-homoiconicity.md) | Chenxu | 2026-02-20 | Permanent Draft ⚠️ |
| RFC-020 | [Dynamic Modules, FFI Integration and Context-Aware Scheduling Enhancement](./draft/020-dynamic-modules-ffi.md) | Chenxu | 2026-02-25 | Draft |
| RFC-021 | [Library-Driven FFI Extension and Cross-Language Call Support](./draft/021-library-driven-ffi-extension.md) | Chenxu | 2026-03-14 | Draft |
| RFC-022 | [Optional Hoare Logic Static Verification (Specification Annotations and Specification Types)](./draft/022-hoare-logic-static-verification.md) | Chenxu | 2026-03-16 | Draft |

---

## RFCs Under Review

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-003 | [Version Planning and Implementation Suggestions](./review/003-version-planning.md) | Chenxu | 2025-01-05 | Under Review |

---

## Accepted RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-001 | [Spawn Model and Error Handling System](./accepted/001-concurrent-model-error-handling.md) | Chenxu | 2025-01-05 | Accepted |
| RFC-004 | [Curried Methods Multi-Position Union Binding Design](./accepted/004-curry-multi-position-binding.md) | Chenxu | 2025-01-05 | Accepted |
| RFC-006 | [Documentation Site Construction and Optimization Plan](./accepted/006-documentation-site-optimization.md) | Chenxu | 2025-01-05 | Accepted |
| RFC-007 | [Function Definition Syntax Unification Scheme](./accepted/007-function-syntax-unification.md) | Chenxu | 2025-01-05 | Accepted |
| RFC-008 | [Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md) | Chenxu | 2025-01-05 | Accepted |
| RFC-009 | [Ownership Model v7](./accepted/009-ownership-model.md) | Chenxu | 2025-01-05 | Accepted |
| RFC-011 | [Generic Type System Design - Zero-Cost Abstraction and Macro Replacement](./accepted/011-generic-type-system.md) | Chenxu | 2025-01-25 | Accepted |
| RFC-012 | [F-String Template Strings](./accepted/012-f-string-template-strings.md) | Chenxu | 2025-01-27 | Accepted |
| RFC-013 | [Error Code Specification Design](./accepted/013-error-code-specification.md) | Chenxu | 2025-01-30 | Accepted |
| RFC-014 | [Package Manager System Design](./accepted/014-package-manager.md) | Chenxu | 2026-02-12 | Accepted |
| RFC-015 | [YaoXiang Configuration System Design](./accepted/015-configuration-system.md) | Chenxu | 2026-02-12 | Accepted |
| RFC-017 | [Language Server Protocol (LSP) Support Design](./review/017-lsp-support.md) | Chenxu | 2026-02-15 | Accepted |

---

## Rejected RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-002 | [Cross-Platform I/O and libuv Integration](./rejected/002-cross-platform-io-libuv.md) | Chenxu | 2025-01-05 | Rejected |
| RFC-005 | [Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md) | Chenxu | 2025-01-05 | Rejected |

---

## RFC Lifecycle

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Open for community discussion and feedback
│  Review     │
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  accepted/  │    │  rejected/  │
│ (official   │    │ (rejected)  │
│  design)    │    │             │
└─────────────┘    └─────────────┘
```

### Status Descriptions

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/reference/rfc/draft/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/reference/rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `docs/reference/rfc/accepted/` | Becomes official design document, enters implementation phase |
| **Rejected** | `docs/reference/rfc/rejected/` | Rejected RFC documents |

---

## Submitting an RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand format requirements
2. Refer to [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the writing style
3. Create a new file, named `number-descriptive-title.md`
4. Place the file in the `docs/reference/rfc/draft/` directory
5. Update this index file, adding the new RFC entry
6. Submit a PR to enter the review process

---

## Contributing Guide

Please refer to [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for the contributing guide.