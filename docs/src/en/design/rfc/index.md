---
title: "RFC Index"
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for design proposals of YaoXiang language features.

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
|------|------|
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md) | RFC standard template |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete example (pattern matching enhancement) |

---

## Draft RFCs

| Number | Title | Author | Created | Status |
|------|------|------|----------|------|
| RFC-019 | [RFC-019: Typed Homoiconicity — Syntax as Types](./draft/019-typed-homoiconicity.md) | Chen Xu | 2026-02-20 | Draft |
| RFC-028 | [RFC-028: JIT Compiler — Multi-level Execution Engine in VM](./draft/028-jit-compiler.md) | Chen Xu | 2026-06-11 | Draft |
| RFC-029 | [RFC-029: Module Semantics System](./draft/029-module-semantics.md) | Chen Xu | 2026-06-13 | Draft |
| RFC-030 | [RFC-030: assert Mechanism](./draft/030-assert-mechanism.md) | Chen Xu | 2026-06-15 | Draft |
| RFC-031 | [RFC-031: Optimization Levels and Pass Manager](./draft/031-optimization-levels.md) | Chen Xu | 2026-06-16 | Draft |
| RFC-032 | [RFC-032: spawn Unified Expression Modifier](./draft/032-spawn-unified-expression.md) | Chen Xu | 2026-06-16 | Draft |

---

## RFCs Under Review

| Number | Title | Author | Created | Status |
|------|------|------|----------|------|
| RFC-025 | [RFC-025: Extensible Primitive Type Mechanism](./review/025-primitive-extension.md) | Chen Xu | 2026-06-05 | Under Review |
| RFC-026 | [RFC-026: FFI Core Mechanism](./review/026-ffi-core-mechanism.md) | Chen Xu | 2026-06-05 | Under Review |

---

## Accepted RFCs

| Number | Title | Author | Created | Status |
|------|------|------|----------|------|
| RFC-004 | [RFC-004: Multi-Position Union Binding Design for Curried Methods](./accepted/004-curry-multi-position-binding.md) | Chen Xu | 2025-01-05 | Accepted |
| RFC-006 | [RFC-006: Documentation Site Construction](./accepted/006-documentation-site-optimization.md) | Chen Xu | 2025-01-05 | Accepted |
| RFC-007 | [RFC-007: Unified Function Definition Syntax](./accepted/007-function-syntax-unification.md) | Mo Yu Jiang | 2025-01-05 | Accepted |
| RFC-008 | [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md) | Chen Xu | 2025-01-05 | Accepted |
| RFC-009 | [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md) | Chen Xu | 2025-01-08 | Accepted |
| ↳ RFC-009a | [RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline](./accepted/009a-borrow-proof-pipeline.md) | Chen Xu | 2026-06-13 | Accepted |
| RFC-010 | [RFC-010: Unified Type Syntax — name: type = value Model](./accepted/010-unified-type-syntax.md) | Chen Xu | 2025-01-20 | Accepted |
| RFC-011 | [RFC-011: Generics System Design — Zero-Cost Abstraction and Macro Alternative](./accepted/011-generic-type-system.md) | Chen Xu | 2025-01-25 | Accepted |
| ↳ RFC-011a | [RFC-011a: Interface Implementation and Dynamic Dispatch](./draft/011a-interface-implementation.md) | Chen Xu | 2026-06-14 | Draft RFC |
| RFC-012 | [RFC-012: F-String Template Strings](./accepted/012-f-string-template-strings.md) | Chen Xu | 2025-01-27 | Accepted |
| RFC-013 | [RFC-013: Error Code Specification](./accepted/013-error-code-specification.md) | Chen Xu | 2026-02-02 | Accepted |
| RFC-014 | [RFC-014: Package Management System Design](./accepted/014-package-manager.md) | Chen Xu | 2026-02-12 | Accepted |
| ↳ RFC-014a | [RFC-014a: Registry Protocol Specification](./draft/014a-registry-protocol.md) | Chen Xu | 2026-06-11 | Draft RFC |
| ↳ RFC-014b | [RFC-014b: Build System and Binary Distribution](./draft/014b-build-system.md) | Chen Xu | 2026-06-11 | Draft RFC |
| ↳ RFC-014c | [RFC-014c: Workspace Support](./draft/014c-workspace.md) | Chen Xu | 2026-06-11 | Draft RFC |
| RFC-015 | [RFC-015: YaoXiang Configuration System Design](./accepted/015-configuration-system.md) | Chen Xu | 2026-02-12 | Accepted |
| RFC-017 | [RFC-017: Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md) | Chen Xu | 2026-02-15 | Under Review |
| RFC-018 | [RFC-018: LLVM AOT Compiler Design](./accepted/018-llvm-aot-compiler.md) | Chen Xu | 2026-02-15 | Accepted |
| RFC-023 | [RFC-023: Closure Capture Model](./accepted/023-closure-capture-model.md) | Chen Xu | 2026-05-29 | Accepted |
| RFC-024 | [RFC-024: Concurrency Model Based on spawn Block](./accepted/024-concurrency-model.md) | Chen Xu | 2026-06-05 | Accepted |
| RFC-027 | [RFC-027: Compile-time Predicates and Unified Static Verification](./accepted/027-compile-time-evaluation-types.md) | Chen Xu | 2026-06-07 | Accepted |

---

## Deprecated RFCs

| Number | Title | Author | Created | Status |
|------|------|------|----------|------|
| RFC-001 | [RFC-001: Concurrency Model and Error Handling System](./deprecated/001-concurrent-model-error-handling.md) | Chen Xu | 2025-01-05 | Deprecated (replaced by RFC-024) |
| RFC-020 | [RFC-020: Dynamic Modules and FFI Integration](./deprecated/020-dynamic-modules-ffi.md) | Chen Xu (based on community discussions) | 2026-03-14 | Deprecated |
| RFC-021 | [RFC-021: Library-driven FFI Extension and Cross-language Call Support](./deprecated/021-library-driven-ffi-extension.md) | Chen Xu | 2026-03-14 | Deprecated |
| RFC-022 | [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](./deprecated/022-hoare-logic-static-verification.md) | Chen Xu | 2026-03-16 | Deprecated (replaced by RFC-027) |

---

## Rejected RFCs

| Number | Title | Author | Created | Status |
|------|------|------|----------|------|
| RFC-002 | [RFC-002: Cross-platform I/O and libuv Integration](./rejected/002-cross-platform-io-libuv.md) | Chen Xu | 2025-01-05 | Rejected |
| RFC-003 | [RFC-003: Version Planning](./rejected/003-version-planning.md) | Chen Xu | 2025-01-05 | Rejected |
| RFC-005 | [RFC-005: Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md) | Chen Xu | 2025-01-05 | Rejected |
| RFC-016 | [RFC-016: Quantum-native Support and Multi-backend Integration](./rejected/016-quantum-native-support.md) | Chen Xu | 2026-02-13 | Rejected |

---

## RFC Lifecycle

```
Draft → Under Review → Accepted → Deprecated (replaced)
                            ↓
                        Rejected (not approved)
```

### Status Description

| Status | Location | Description |
|------|------|------|
| **Draft** | `rfc/draft/` | Author's draft, waiting to be submitted for review |
| **Under Review** | `rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `rfc/accepted/` | Becomes a formal design document and enters the implementation phase |
| **Deprecated** | `rfc/deprecated/` | Previously accepted, now superseded by a new design |
| **Rejected** | `rfc/rejected/` | RFC documents that have been rejected |

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