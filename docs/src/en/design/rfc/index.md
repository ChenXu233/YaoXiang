---
title: "RFC Index"
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for YaoXiang language feature design proposals.

## Table of Contents

- [Template](#template)
- [Draft RFCs](#draft-rfcs)
- [RFCs Under Review](#rfcs-under-review)
- [Accepted RFCs](#accepted-rfcs)
- [Deprecated RFCs](#deprecated-rfcs)
- [Rejected RFCs](#rejected-rfcs)

---

## Template

| File | Description |
|------|-------------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC Standard Template |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete Example (Pattern Matching Enhancement) |

---

## Draft RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-019 | [Typed Homoiconicity](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | Permanent Draft ⚠️ |
| RFC-020 | [Dynamic Modules, FFI Integration and Context-Aware Scheduling Enhancement](./draft/020-dynamic-modules-ffi.md) | 晨煦 | 2026-02-25 | Draft |
| RFC-025 | [Extensible Primitive Type Mechanism](./draft/025-primitive-extension.md) | 晨煦 | 2026-06-05 | Draft |

---

## RFCs Under Review

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-018 | [LLVM AOT Compiler and Runtime Scheduler Integration Design](./review/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | Under Review |
| RFC-021 | [Library-Driven FFI Extension and Cross-Language Invocation Support](./review/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | Under Review |
| RFC-022 | [Optional Hoare Logic Static Verification (Specification Comments and Specification Types)](./review/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | Under Review |

---

## Accepted RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-001 | [Concurrency Model and Error Handling System](./accepted/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-004 | [Multi-Position Union Binding Design for Curried Methods](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-006 | [Documentation Site Construction and Optimization Plan](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-007 | [Function Definition Syntax Unification](./accepted/007-function-syntax-unification.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-008 | [Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-009 | [Ownership Model v7](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-010 | [Unified Type Syntax](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-25 | Accepted |
| RFC-011 | [Generic Type System Design - Zero-Cost Abstraction and Macro Substitution](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | Accepted |
| RFC-012 | [F-String Template Strings](./accepted/012-f-string-template-strings.md) | 晨煦 | 2025-01-27 | Accepted |
| RFC-013 | [Error Code Specification Design](./accepted/013-error-code-specification.md) | 晨煦 | 2025-01-30 | Accepted |
| RFC-014 | [Package Management System Design](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | Accepted |
| RFC-015 | [YaoXiang Configuration System Design](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | Accepted |
| RFC-017 | [Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | Accepted |
| RFC-023 | [Closure Capture Model](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | Accepted |
| RFC-024 | [Concurrency Model Based on Spawn Blocks](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | Accepted |

---

## Deprecated RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| (None) | | | | |

---

## Rejected RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-002 | [Cross-Platform I/O and libuv Integration](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | Rejected |
| RFC-003 | [Version Planning and Implementation Suggestions](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | Rejected |
| RFC-005 | [Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | Rejected |
| RFC-016 | [Quantum-Native Support and Multi-Backend Integration](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | Rejected |

---

## RFC Lifecycle

```
Draft → Under Review → Accepted → Deprecated (replaced by newer design)
                    ↓
                Rejected (not approved)
```

### Status Description

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `rfc/draft/` | Author's draft, awaiting submission for review |
| **Under Review** | `rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `rfc/accepted/` | Becomes an official design document, entering implementation phase |
| **Deprecated** | `rfc/deprecated/` | Was once accepted, replaced by a newer design |
| **Rejected** | `rfc/rejected/` | RFC documents that were rejected |

---

## Submitting an RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand the format requirements
2. Refer to [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the writing style
3. Create a new file named `number-descriptive-title.md`
4. Place the file in the `docs/reference/rfc/draft/` directory
5. Update this index file to add the new RFC entry
6. Submit a PR to enter the review process

---

## Contribution Guidelines

Please refer to [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for contribution guidelines.