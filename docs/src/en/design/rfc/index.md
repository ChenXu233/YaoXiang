---
title: 'RFC Index'
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for design proposals of YaoXiang
> language features.

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
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | RFC standard template                           |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete example (Pattern Matching Enhancement) |

---

## Draft RFCs

| Number   | Title                                                                                                    | Author  | Created    | Status       |
| -------- | -------------------------------------------------------------------------------------------------------- | ------- | ---------- | ------------ |
| RFC-002  | [RFC-002: libuv-based Resource Type IO Implementation Layer](./draft/002-cross-platform-io-libuv.md)     | Chen Xu | 2026-01-05 | Draft        |
| RFC-019  | [RFC-019: Typed Homoiconicity - Syntax is Type](./draft/019-typed-homoiconicity.md)                      | Chen Xu | 2026-02-20 | Draft        |
| RFC-028  | [RFC-028: JIT Compiler — Multi-level Execution Engine within VM](./draft/028-jit-compiler.md)            | Chen Xu | 2026-06-11 | Draft        |
| RFC-031  | [RFC-031: Optimization Levels and Pass Manager](./draft/031-optimization-levels.md)                      | Chen Xu | 2026-06-16 | Draft        |
| RFC-033  | [RFC-033: `^^` Reflection Operator](./draft/033-reflection-operator.md)                                  | Chen Xu | 2026-06-16 | Under Review |
| RFC-034  | [RFC-034: Unified Debugging Toolchain](./draft/034-debug-toolchain.md)                                   | Chen Xu | 2026-07-06 | Draft        |
| RFC-035  | [RFC-035: MCP Server Support (AI Agent Integration)](./draft/035-mcp-server.md)                          | Chen Xu | 2026-07-11 | Draft        |
| RFC-039  | [RFC-039: Pattern Matching Completeness](./draft/039-pattern-matching-completeness.md)                   | Chen Xu | 2026-09-03 | Draft        |
| RFC-027a | [RFC-027a: Proof Function Fallback for Termination Checking](./draft/027a-termination-proof-fallback.md) | Chen Xu | 2026-09-14 | Draft        |
| RFC-029a | [RFC-029a: Module Cache and Incremental Recompilation](./draft/029a-module-cache-incremental.md)         | Chen Xu | 2026-09-07 | Draft        |

---

## RFCs Under Review

| Number  | Title                                                                                                                           | Author  | Created    | Status       |
| ------- | ------------------------------------------------------------------------------------------------------------------------------- | ------- | ---------- | ------------ |
| RFC-032 | [RFC-032: spawn Unified Expression Modifier — Eliminating the spawn for Special Case](./review/032-spawn-unified-expression.md) | Chen Xu | 2026-06-16 | Under Review |

---

## Accepted RFCs

| Number     | Title                                                                                                                                  | Author    | Created    | Status             |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------- | ---------- | ------------------ |
| RFC-004    | [RFC-004: Multi-position Union Binding Design for Curried Methods](./accepted/004-curry-multi-position-binding.md)                     | Chen Xu   | 2025-01-05 | Accepted           |
| RFC-006    | [RFC-006: Documentation Site Construction](./accepted/006-documentation-site-optimization.md)                                          | Chen Xu   | 2025-01-05 | Accepted           |
| RFC-007    | [RFC-007: Function Definition Syntax Unification Plan](./accepted/007-function-syntax-unification.md)                                  | 沫郁酱    | 2025-01-05 | Accepted           |
| RFC-008    | [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md)                      | Chen Xu   | 2025-01-05 | Accepted           |
| RFC-009    | [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md)                                                                   | Chen Xu   | 2025-01-08 | Accepted           |
| ↳ RFC-009a | [RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline Based](./accepted/009a-borrow-proof-pipeline.md)                             | Chen Xu   | 2026-06-13 | Accepted           |
| RFC-010    | [RFC-010: Unified Type Syntax - name: type = value Model](./accepted/010-unified-type-syntax.md)                                       | Chen Xu   |            | Accepted           |
| ↳ RFC-010a | [RFC-010a: Tail Expression Evaluation and return Semantics](./accepted/010a-tail-expression-and-return.md)                             | Chen Xu   | 2026-09-15 | Accepted           |
| RFC-011    | [RFC-011: Generic System Design - Zero-cost Abstraction and Macro Replacement](./accepted/011-generic-type-system.md)                  | Chen Xu   |            | Accepted           |
| ↳ RFC-011a | [RFC-011a: Interface Implementation and Dynamic Dispatch](./accepted/011a-interface-implementation.md)                                 | Chen Xu   | 2026-06-14 | Accepted           |
| ↳ RFC-011b | [RFC-011b: Operator Overloading and Interface-driven Operators](./draft/011b-operator-overloading.md)                                  | Chen Xu   | 2026-09-15 | Draft RFC          |
| RFC-012    | [RFC-012: F-String Template Strings](./accepted/012-f-string-template-strings.md)                                                      | Chen Xu   | 2025-01-27 | Accepted           |
| RFC-013    | [RFC-013: Error Code Specification](./accepted/013-error-code-specification.md)                                                        | Chen Xu   | 2026-02-02 | Accepted           |
| RFC-014    | [RFC-014: Package Management System Design](./accepted/014-package-manager.md)                                                         | Chen Xu   | 2026-02-12 | Accepted           |
| ↳ RFC-014a | [RFC-014a: Registry Protocol Specification](./review/014a-registry-protocol.md)                                                        | Chen Xu   | 2026-06-11 | Under Review RFC   |
| ↳ RFC-014b | [RFC-014b: Build System and Binary Distribution](./review/014b-build-system.md)                                                        | Chen Xu   | 2026-06-11 | Under Review RFC   |
| ↳ RFC-014c | [RFC-014c: Workspace Support](./review/014c-workspace.md)                                                                              | Chen Xu   | 2026-06-11 | Under Review RFC   |
| RFC-015    | [RFC-015: YaoXiang Configuration System Design](./accepted/015-configuration-system.md)                                                | Chen Xu   | 2026-02-12 | Accepted           |
| RFC-017    | [RFC-017: Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md)                                                | Chen Xu   | 2026-02-15 | Implemented        |
| RFC-018    | [RFC-018: LLVM AOT Compiler Design](./accepted/018-llvm-aot-compiler.md)                                                               | Chen Xu   | 2026-02-15 | Accepted           |
| RFC-024    | [RFC-024: spawn-based Concurrency Runtime Semantics](./accepted/024-concurrency-model.md)                                              | Chen Xu   | 2026-06-05 | Accepted (Revised) |
| RFC-026    | [RFC-026: FFI Core Mechanism](./accepted/026-ffi-core-mechanism.md)                                                                    | Chen Xu   | 2026-07-03 | Accepted           |
| ↳ RFC-026a | [RFC-026a: Extensible FFI Mechanism System](./review/026a-extensible-ffi-system.md)                                                    | Chen Xu   | 2026-06-05 | Under Review RFC   |
| ↳ RFC-026b | [RFC-026b: yx-bindgen Toolchain](./draft/026b-yx-bindgen.md)                                                                           | Chen Xu   | 2026-06-05 | Draft RFC          |
| RFC-027    | [RFC-027: Compile-time Predicates and Unified Static Verification](./accepted/027-compile-time-evaluation-types.md)                    | Chen Xu   | 2026-06-07 | Accepted           |
| RFC-029    | [RFC-029: Module Semantics System](./accepted/029-module-semantics.md)                                                                 | Chen Xu   | 2026-06-13 | Accepted           |
| RFC-030    | [RFC-030: assert Assertion Mechanism](./accepted/030-assert-mechanism.md)                                                              | Chen Xu   | 2026-06-15 | Accepted           |
| RFC-036    | [RFC-036: std.test Testing Framework and yaoxiang test Command](./accepted/036-test-framework.md)                                      | Chen Xu   | 2026-07-26 | Accepted           |
| RFC-037    | [RFC-037: Industrial Distribution Solution — Compiler/Toolchain Packaging Based on cargo-dist](./accepted/037-industrial-packaging.md) | ChenXu233 | 2026-07-26 | Accepted           |
| RFC-038    | [RFC-038: Statement Termination & Newline Rules](./accepted/038-statement-termination.md)                                              | ChenXu233 | 2026-08-05 | Accepted           |
| RFC-029f   | [RFC-029f: Compilation Target Roles and Import Surface Semantics](./accepted/029f-target-semantics.md)                                 | Chen Xu   | 2026-09-12 | Accepted           |

---

## Deprecated RFCs

| Number  | Title                                                                                                                                                       | Author  | Created    | Status                             |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ---------- | ---------------------------------- |
| RFC-001 | [RFC-001: spawn Model and Error Handling System](./deprecated/001-concurrent-model-error-handling.md)                                                       | Chen Xu | 2025-01-05 | Deprecated (Superseded by RFC-024) |
| RFC-020 | [RFC-020: Dynamic Modules and FFI Integration](./deprecated/020-dynamic-modules-ffi.md)                                                                     | Chen Xu | 2026-03-14 | Deprecated                         |
| RFC-021 | [RFC-021: Library-driven FFI Extension and Cross-language Call Support](./deprecated/021-library-driven-ffi-extension.md)                                   | Chen Xu | 2026-03-14 | Deprecated                         |
| RFC-022 | [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](./deprecated/022-hoare-logic-static-verification.md) | Chen Xu | 2026-03-16 | Deprecated (Superseded by RFC-027) |
| RFC-023 | [RFC-023: Closure Capture Model](./deprecated/023-closure-capture-model.md)                                                                                 | Chen Xu | 2026-05-29 | Deprecated                         |

---

## Rejected RFCs

| Number  | Title                                                                                                     | Author  | Created    | Status   |
| ------- | --------------------------------------------------------------------------------------------------------- | ------- | ---------- | -------- |
| RFC-003 | [RFC-003: Version Planning](./rejected/003-version-planning.md)                                           | Chen Xu | 2025-01-05 | Rejected |
| RFC-005 | [RFC-005: Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md)               | Chen Xu | 2025-01-05 | Rejected |
| RFC-016 | [RFC-016: Quantum Native Support and Multi-backend Integration](./rejected/016-quantum-native-support.md) | Chen Xu | 2026-02-13 | Rejected |
| RFC-025 | [RFC-025: Extensible Primitive Type Mechanism](./rejected/025-primitive-extension.md)                     | Chen Xu | 2026-06-05 | Rejected |

---

## RFC Lifecycle

```
Draft → Under Review → Accepted → Deprecated (Superseded)
                            ↓
                       Rejected (Not Approved)
```

### Status Description

| Status           | Location          | Description                                                   |
| ---------------- | ----------------- | ------------------------------------------------------------- |
| **Draft**        | `rfc/draft/`      | Author's draft, awaiting submission for review                |
| **Under Review** | `rfc/review/`     | Open for community discussion and feedback                    |
| **Accepted**     | `rfc/accepted/`   | Becomes official design document, enters implementation phase |
| **Deprecated**   | `rfc/deprecated/` | Previously accepted, superseded by new design                 |
| **Rejected**     | `rfc/rejected/`   | Rejected RFC documents                                        |

---

## Submitting an RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand format requirements
2. Refer to [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the
   writing style
3. Create a new file named `number-descriptive-title.md`
4. Place the file in the `docs/reference/rfc/draft/` directory
5. Update this index file to add the new RFC entry
6. Submit a PR to enter the review process

---

## Contribution Guidelines

Please refer to [CONTRIBUTING.md](../../../../CONTRIBUTING.md) for contribution guidelines.
