---
title: "RFC Index"
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for YaoXiang language feature design proposals.

## Table of Contents

- [Template](#template)
- [Draft RFCs](#draft-rfcs)
- [Under Review RFCs](#under-review-rfcs)
- [Accepted RFCs](#accepted-rfcs)
- [Deprecated RFCs](#deprecated-rfcs)
- [Rejected RFCs](#rejected-rfcs)

---

## Template

| File | Description |
|------|-------------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC Standard Template |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Full Example (Pattern Matching Enhancement) |

---

## Draft RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-019 | [RFC-019: Type-Level YaoXiang (Typed Homoiconicity) - Syntax as Types](./draft/019-typed-homoiconicity.md) | 晨煦 | 2026-02-20 | Draft |
| RFC-025 | [RFC-025: Extensible Primitive Type Mechanism](./draft/025-primitive-extension.md) | 晨煦 | 2026-06-05 | Draft |

---

## Under Review RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-018 | [RFC-018: LLVM AOT Compiler Design](./review/018-llvm-aot-compiler.md) | 晨煦 | 2026-02-15 | Under Review |
| RFC-026 | [RFC-026: FFI Core Mechanism](./review/026-ffi-core-mechanism.md) | 晨煦 | 2026-06-05 | Under Review |
| RFC-027 | [RFC-027: Compile-Time Evaluation Types and Unified Static Verification](./review/027-compile-time-evaluation-types.md) | 晨煦 | 2026-06-07 | Under Review |

---

## Accepted RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-004 | [RFC-004: Multi-Position Union Binding Design for Curried Methods](./accepted/004-curry-multi-position-binding.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-006 | [RFC-006: Documentation Site Construction](./accepted/006-documentation-site-optimization.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-007 | [RFC-007: Function Definition Syntax Unification](./accepted/007-function-syntax-unification.md) | 沫郁酱 | 2025-01-05 | Accepted |
| RFC-008 | [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md) | 晨煦 | 2025-01-05 | Accepted |
| RFC-009 | [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md) | 晨煦 | 2025-01-08 | Accepted |
| RFC-010 | [RFC-010: Unified Type Syntax - name: type = value Model](./accepted/010-unified-type-syntax.md) | 晨煦 | 2025-01-20 | Accepted |
| RFC-011 | [RFC-011: Generic System Design - Zero-Cost Abstraction and Macro Substitution](./accepted/011-generic-type-system.md) | 晨煦 | 2025-01-25 | Accepted |
| RFC-012 | [RFC 012: F-String Template Strings](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | Accepted |
| RFC-013 | [RFC 013: Error Code Specification](./accepted/013-error-code-specification.md) | 晨煦 | 2026-02-02 | Accepted |
| RFC-014 | [RFC-014: Package Manager Design](./accepted/014-package-manager.md) | 晨煦 | 2026-02-12 | Accepted |
| RFC-015 | [RFC-015: YaoXiang Configuration System Design](./accepted/015-configuration-system.md) | 晨煦 | 2026-02-12 | Accepted |
| RFC-017 | [RFC-017: Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md) | 晨煦 | 2026-02-15 | Under Review |
| RFC-023 | [RFC-023: Closure Capture Model](./accepted/023-closure-capture-model.md) | 晨煦 | 2026-05-29 | Accepted |
| RFC-024 | [RFC-024: Concurrency Model Based on Spawn Blocks](./accepted/024-concurrency-model.md) | 晨煦 | 2026-06-05 | Accepted |

---

## Deprecated RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-001 | [RFC-001: Spawn Model and Error Handling System](./deprecated/001-concurrent-model-error-handling.md) | 晨煦 | 2025-01-05 | Deprecated (superseded by RFC-024) |
| RFC-020 | [RFC-020: Dynamic Modules and FFI Integration](./deprecated/020-dynamic-modules-ffi.md) | 晨煦 (organized based on community discussions) | 2026-03-14 | Deprecated |
| RFC-021 | [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](./deprecated/021-library-driven-ffi-extension.md) | 晨煦 | 2026-03-14 | Deprecated |
| RFC-022 | [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](./deprecated/022-hoare-logic-static-verification.md) | 晨煦 | 2026-03-16 | Deprecated (superseded by RFC-027) |

---

## Rejected RFCs

| Number | Title | Author | Created | Status |
|--------|-------|--------|---------|--------|
| RFC-002 | [RFC-002: Cross-Platform I/O and libuv Integration](./rejected/002-cross-platform-io-libuv.md) | 晨煦 | 2025-01-05 | Rejected |
| RFC-003 | [RFC-003: Version Planning](./rejected/003-version-planning.md) | 晨煦 | 2025-01-05 | Rejected |
| RFC-005 | [RFC-005: Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md) | 晨煦 | 2025-01-05 | Rejected |
| RFC-016 | [RFC 016: Quantum-Native Support and Multi-Backend Integration](./rejected/016-quantum-native-support.md) | 晨煦 | 2026-02-13 | Rejected |

---

## RFC Lifecycle

```
Draft → Under Review → Accepted → Deprecated (superseded)
                              ↓
                          Rejected (not approved)
```

### Status Descriptions

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `rfc/draft/` | Author's draft, awaiting submission for review |
| **Under Review** | `rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `rfc/accepted/` | Becomes a formal design document, enters implementation phase |
| **Deprecated** | `rfc/deprecated/` | Was previously accepted, superseded by new design |
| **Rejected** | `rfc/rejected/` | RFC documents that were rejected |

---

## Submitting an RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand format requirements
2. Reference [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the writing style
3. Create a new file, naming it `number-descriptive-title.md`
4. Place the file in the `docs/reference/rfc/draft/` directory
5. Update this index file to add the new RFC entry
6. Submit a PR to enter the review process

---

## Contribution Guidelines

Please refer to [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for contribution guidelines.