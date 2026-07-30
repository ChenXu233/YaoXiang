---
title: 'RFC Index'
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for YaoXiang language feature design
> proposals.

## Table of Contents

- [Templates](#templates)
- [Draft RFCs](#draft-rfcs)
- [RFCs Under Review](#rfcs-under-review)
- [Accepted RFCs](#accepted-rfcs)
- [Deprecated RFCs](#deprecated-rfcs)
- [Rejected RFCs](#rejected-rfcs)

---

## Templates

| File                                                                 | Description                                     |
| -------------------------------------------------------------------- | ----------------------------------------------- |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | Standard RFC template                           |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete example (pattern matching enhancement) |

---

## Draft RFCs

| Number   | Title                                                                                                                      | Author    | Date       | Status            |
| -------- | -------------------------------------------------------------------------------------------------------------------------- | --------- | ---------- | ----------------- |
| RFC-019  | [RFC-019: Type-Level Homoiconicity — Syntax as Type](./draft/019-typed-homoiconicity.md)                                   | 晨煦      | 2026-02-20 | Draft             |
| RFC-028  | [RFC-028: JIT Compiler — Multi-Level Execution Engine in VM](./draft/028-jit-compiler.md)                                  | 晨煦      | 2026-06-11 | Draft             |
| RFC-031  | [RFC-031: Optimization Levels and Pass Manager](./draft/031-optimization-levels.md)                                        | 晨煦      | 2026-06-16 | Draft             |
| RFC-002  | [RFC-002: Resource Type IO Implementation Layer Based on libuv](./draft/002-cross-platform-io-libuv.md)                    | 晨煦      | 2025-01-05 | Draft (Re-review) |
| RFC-026b | [RFC-026b: yx-bindgen Toolchain](./draft/026b-yx-bindgen.md)                                                               | 晨煦      | 2026-07-03 | Draft             |
| RFC-034  | [RFC-034: Unified Debug Toolchain](./draft/034-debug-toolchain.md)                                                         | 晨煦      | 2026-07-06 | Draft             |
| RFC-035  | [RFC-035: MCP Server Support (AI Agent Integration)](./draft/035-mcp-server.md)                                            | Chen Xu   | 2026-07-11 | Draft             |
| RFC-036  | [RFC-036: std.test Testing Framework and yaoxiang test Command](./draft/036-test-framework.md)                             | 晨煦      | 2026-07-25 | Draft             |
| RFC-037  | [RFC-037: Industrial Distribution — Compiler/Toolchain Packaging Based on cargo-dist](./draft/037-industrial-packaging.md) | ChenXu233 | 2026-07-26 | Draft             |

---

## RFCs Under Review

| Number   | Title                                                                                                                            | Author | Date       | Status       |
| -------- | -------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------- | ------------ |
| RFC-026a | [RFC-026a: Extensible FFI Mechanism System](./review/026a-extensible-ffi-system.md)                                              | 晨煦   | 2026-07-03 | Under Review |
| RFC-032  | [RFC-032: Unified Expression Modifier for spawn — Eliminating spawn for Special Cases](./review/032-spawn-unified-expression.md) | 晨煦   | 2026-06-16 | Under Review |

---

## Accepted RFCs

| Number     | Title                                                                                                                      | Author  | Date       | Status       |
| ---------- | -------------------------------------------------------------------------------------------------------------------------- | ------- | ---------- | ------------ |
| RFC-004    | [RFC-004: Curry Method Multi-Position Union Binding Design](./accepted/004-curry-multi-position-binding.md)                | 晨煦    | 2025-01-05 | Accepted     |
| RFC-006    | [RFC-006: Documentation Site Construction](./accepted/006-documentation-site-optimization.md)                              | 晨煦    | 2025-01-05 | Accepted     |
| RFC-007    | [RFC-007: Function Definition Syntax Unification](./accepted/007-function-syntax-unification.md)                           | 沫郁酱  | 2025-01-05 | Accepted     |
| RFC-008    | [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md)          | 晨煦    | 2025-01-05 | Accepted     |
| RFC-009    | [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md)                                                       | 晨煦    | 2025-01-08 | Accepted     |
| ↳ RFC-009a | [RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline](./accepted/009a-borrow-proof-pipeline.md)                       | 晨煦    | 2026-06-13 | Accepted     |
| RFC-010    | [RFC-010: Unified Type Syntax — `name: type = value` Model](./accepted/010-unified-type-syntax.md)                         | 晨煦    | 2025-01-20 | Accepted     |
| RFC-011    | [RFC-011: Generic Type System Design — Zero-Cost Abstraction and Macro Alternative](./accepted/011-generic-type-system.md) | 晨煦    | 2025-01-25 | Accepted     |
| ↳ RFC-011a | [RFC-011a: Interface Implementation and Dynamic Dispatch](./review/011a-interface-implementation.md)                       | 晨煦    | 2026-06-14 | Under Review |
| RFC-012    | [RFC-012: F-String Template Strings](./accepted/012-f-string-template-strings.md)                                          | Chen Xu | 2025-01-27 | Accepted     |
| RFC-013    | [RFC-013: Error Code Specification](./accepted/013-error-code-specification.md)                                            | 晨煦    | 2026-02-02 | Accepted     |
| RFC-014    | [RFC-014: Package Management System Design](./accepted/014-package-manager.md)                                             | 晨煦    | 2026-02-12 | Accepted     |
| ↳ RFC-014a | [RFC-014a: Registry Protocol Specification](./review/014a-registry-protocol.md)                                            | 晨煦    | 2026-06-11 | Under Review |
| ↳ RFC-014b | [RFC-014b: Build System and Binary Distribution](./review/014b-build-system.md)                                            | 晨煦    | 2026-06-11 | Under Review |
| ↳ RFC-014c | [RFC-014c: Workspace Support](./review/014c-workspace.md)                                                                  | 晨煦    | 2026-06-11 | Under Review |
| RFC-015    | [RFC-015: YaoXiang Configuration System Design](./accepted/015-configuration-system.md)                                    | 晨煦    | 2026-02-12 | Accepted     |
| RFC-017    | [RFC-017: Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md)                                    | 晨煦    | 2026-02-15 | Under Review |
| RFC-018    | [RFC-018: LLVM AOT Compiler Design](./accepted/018-llvm-aot-compiler.md)                                                   | 晨煦    | 2026-02-15 | Accepted     |
| RFC-024    | [RFC-024: Concurrency Model Based on spawn Block](./accepted/024-concurrency-model.md)                                     | 晨煦    | 2026-06-05 | Accepted     |
| RFC-026    | [RFC-026: FFI Core Mechanism](./accepted/026-ffi-core-mechanism.md)                                                        | 晨煦    | 2026-06-05 | Accepted     |
| RFC-027    | [RFC-027: Compile-Time Predicates and Unified Static Verification](./accepted/027-compile-time-evaluation-types.md)        | 晨煦    | 2026-06-07 | Accepted     |
| RFC-030    | [RFC-030: assert Assertion Mechanism](./accepted/030-assert-mechanism.md)                                                  | 晨煦    | 2026-06-15 | Accepted     |
| RFC-029    | [RFC-029: Module Semantics System](./accepted/029-module-semantics.md)                                                     | 晨煦    | 2026-06-13 | Accepted     |

---

## Deprecated RFCs

| Number  | Title                                                                                                                                                    | Author | Date       | Status                             |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------- | ---------------------------------- |
| RFC-001 | [RFC-001: spawn Model and Error Handling System](./deprecated/001-concurrent-model-error-handling.md)                                                    | 晨煦   | 2025-01-05 | Deprecated (Superseded by RFC-024) |
| RFC-020 | [RFC-020: Dynamic Modules and FFI Integration](./deprecated/020-dynamic-modules-ffi.md)                                                                  | 晨煦   | 2026-03-14 | Deprecated                         |
| RFC-021 | [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](./deprecated/021-library-driven-ffi-extension.md)                                | 晨煦   | 2026-03-14 | Deprecated                         |
| RFC-022 | [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](./deprecated/022-hoare-logic-static-verification.md) | 晨煦   | 2026-03-16 | Deprecated (Superseded by RFC-027) |
| RFC-023 | [RFC-023: Closure Capture Model](./deprecated/023-closure-capture-model.md)                                                                              | 晨煦   | 2026-05-29 | Deprecated                         |

---

## Rejected RFCs

| Number  | Title                                                                                                     | Author | Date       | Status                                      |
| ------- | --------------------------------------------------------------------------------------------------------- | ------ | ---------- | ------------------------------------------- |
| RFC-003 | [RFC-003: Version Planning](./rejected/003-version-planning.md)                                           | 晨煦   | 2025-01-05 | Rejected                                    |
| RFC-005 | [RFC-005: Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md)               | 晨煦   | 2025-01-05 | Rejected                                    |
| RFC-016 | [RFC-016: Quantum-Native Support and Multi-Backend Integration](./rejected/016-quantum-native-support.md) | 晨煦   | 2026-02-13 | Rejected                                    |
| RFC-025 | [RFC-025: Extensible Primitive Type Mechanism](./rejected/025-primitive-extension.md)                     | 晨煦   | 2026-06-05 | Rejected (Covered by RFC-026 Opaque Handle) |

---

## RFC Lifecycle

```
Draft → Under Review → Accepted → Deprecated (Superseded)
                            ↓
                        Rejected (Not Approved)
```

### Status Description

| Status           | Location          | Description                                   |
| ---------------- | ----------------- | --------------------------------------------- |
| **Draft**        | `rfc/draft/`      | Author's draft, awaiting review submission    |
| **Under Review** | `rfc/review/`     | Open community discussion and feedback        |
| **Deprecated**   | `rfc/deprecated/` | Previously accepted, superseded by new design |
| **Rejected**     | `rfc/rejected/`   | Rejected RFC documents                        |

---

## Submitting an RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand the format requirements
2. Refer to [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the
   writing style
3. Create a new file named `<number>-<descriptive-title>.md`
4. Place the file in the `docs/src/design/rfc/draft/` directory
5. Update this index file to add the new RFC entry
6. Submit a PR to enter the review process

---

## Contribution Guidelines

Please refer to [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for contribution guidelines.
