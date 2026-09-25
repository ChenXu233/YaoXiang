---
title: 'RFC Index'
---

# YaoXiang RFC (Request for Comments) Index

> RFC (Request for Comments) is the formal submission format for YaoXiang language feature design
> proposals.

## Table of Contents

- [Templates](#templates)
- [Draft RFC](#draft-rfc)
- [Under Review](#under-review)
- [Accepted RFC](#accepted-rfc)
- [Deprecated RFC](#deprecated-rfc)
- [Rejected RFC](#rejected-rfc)
- [Document Revision Rules](#document-revision-rules)

---

## Templates

| File                                                                 | Description                                     |
| -------------------------------------------------------------------- | ----------------------------------------------- |
| [RFC_TEMPLATE.md](RFC_TEMPLATE.md)                                   | RFC Standard Template                           |
| [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) | Complete Example (Pattern Matching Enhancement) |

---

## Draft RFC

| Number   | Title                                                                                                   | Author  | Creation Date | Status       |
| -------- | ------------------------------------------------------------------------------------------------------- | ------- | ------------- | ------------ |
| RFC-002  | [RFC-002: Resource-Type IO Implementation Layer Based on libuv](./draft/002-cross-platform-io-libuv.md) | Chen Xu | 2026-01-05    | Draft        |
| RFC-019  | [RFC-019: Typed Homoiconicity - Syntax as Types](./draft/019-typed-homoiconicity.md)                    | Chen Xu | 2026-02-20    | Draft        |
| RFC-028  | [RFC-028: JIT Compiler — Multi-level Execution Engine in VM](./draft/028-jit-compiler.md)               | Chen Xu | 2026-06-11    | Draft        |
| RFC-031  | [RFC-031: Optimization Levels and Pass Manager](./draft/031-optimization-levels.md)                     | Chen Xu | 2026-06-16    | Draft        |
| RFC-033  | [RFC-033: `^^` Reflection Operator](./draft/033-reflection-operator.md)                                 | Chen Xu | 2026-06-16    | Under Review |
| RFC-034  | [RFC-034: Unified Debugging Toolchain](./draft/034-debug-toolchain.md)                                  | Chen Xu | 2026-07-06    | Draft        |
| RFC-035  | [RFC-035: MCP Server Support (AI Agent Integration)](./draft/035-mcp-server.md)                         | Chen Xu | 2026-07-11    | Draft        |
| RFC-027a | [RFC-027a: Explicit Measure for Termination Checking](./review/027a-termination-explicit-measure.md)    | Chen Xu | 2026-09-14    | Under Review |
| RFC-029a | [RFC-029a: Module Cache and Incremental Recompilation](./draft/029a-module-cache-incremental.md)        | Chen Xu | 2026-09-07    | Draft        |

---

## Under Review

| Number  | Title                                                                                                                           | Author  | Creation Date | Status       |
| ------- | ------------------------------------------------------------------------------------------------------------------------------- | ------- | ------------- | ------------ |
| RFC-032 | [RFC-032: Unified spawn Expression Modifier — Eliminating Special Cases of spawn for](./review/032-spawn-unified-expression.md) | Chen Xu | 2026-06-16    | Under Review |

---

## Accepted RFC

| Number     | Title                                                                                                                               | Author    | Creation Date | Status             |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------- | --------- | ------------- | ------------------ |
| RFC-004    | [RFC-004: Multi-Position Joint Binding Design for Curried Methods](./accepted/004-curry-multi-position-binding.md)                  | Chen Xu   | 2025-01-05    | Accepted           |
| RFC-006    | [RFC-006: Documentation Site Construction](./accepted/006-documentation-site-optimization.md)                                       | Chen Xu   | 2025-01-05    | Accepted           |
| RFC-007    | [RFC-007: Function Definition Syntax Unification Plan](./accepted/007-function-syntax-unification.md)                               | 沫郁酱    | 2025-01-05    | Accepted           |
| RFC-008    | [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md)                   | Chen Xu   | 2025-01-05    | Accepted           |
| RFC-009    | [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md)                                                                | Chen Xu   | 2025-01-08    | Accepted           |
| ↳ RFC-009a | [RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline-Based](./accepted/009a-borrow-proof-pipeline.md)                          | Chen Xu   | 2026-06-13    | Accepted           |
| RFC-010    | [RFC-010: Unified Type Syntax - name: type = value Model](./accepted/010-unified-type-syntax.md)                                    | Chen Xu   |               | Accepted           |
| ↳ RFC-010a | [RFC-010a: Tail Expression Evaluation and return Semantics](./accepted/010a-tail-expression-and-return.md)                          | Chen Xu   | 2026-09-15    | Accepted           |
| ↳ RFC-010b | [RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)](./draft/010b-pattern-matching-completeness.md) | Chen Xu   | 2026-09-03    | Draft              |
| RFC-011    | [RFC-011: Generics System Design - Zero-Cost Abstraction and Macro Replacement](./accepted/011-generic-type-system.md)              | Chen Xu   |               | Accepted           |
| ↳ RFC-011a | [RFC-011a: Interface Implementation and Dynamic Dispatch](./accepted/011a-interface-implementation.md)                              | Chen Xu   | 2026-06-14    | Accepted           |
| ↳ RFC-011b | [RFC-011b: Operator Overloading and Interface-Driven Operators](./accepted/011b-operator-overloading.md)                            | Chen Xu   | 2026-09-22    | Accepted           |
| RFC-012    | [RFC 012: F-String Template Strings](./accepted/012-f-string-template-strings.md)                                                   | Chen Xu   | 2025-01-27    | Accepted           |
| RFC-013    | [RFC 013: Error Code Specification](./accepted/013-error-code-specification.md)                                                     | Chen Xu   | 2026-02-02    | Accepted           |
| RFC-014    | [RFC-014: Package Management System Design](./accepted/014-package-manager.md)                                                      | Chen Xu   | 2026-02-12    | Accepted           |
| ↳ RFC-014a | [RFC-014a: Registry Protocol Specification](./review/014a-registry-protocol.md)                                                     | Chen Xu   | 2026-06-11    | Under Review       |
| ↳ RFC-014b | [RFC-014b: Build System and Binary Distribution](./review/014b-build-system.md)                                                     | Chen Xu   | 2026-06-11    | Under Review       |
| ↳ RFC-014c | [RFC-014c: Workspace Support](./review/014c-workspace.md)                                                                           | Chen Xu   | 2026-06-11    | Under Review       |
| RFC-015    | [RFC-015: YaoXiang Configuration System Design](./accepted/015-configuration-system.md)                                             | Chen Xu   | 2026-02-12    | Accepted           |
| RFC-017    | [RFC-017: Language Server Protocol (LSP) Support Design](./accepted/017-lsp-support.md)                                             | Chen Xu   | 2026-02-15    | Implemented        |
| RFC-018    | [RFC-018: LLVM AOT Compiler Design](./accepted/018-llvm-aot-compiler.md)                                                            | Chen Xu   | 2026-02-15    | Accepted           |
| RFC-024    | [RFC-024: Concurrency Runtime Semantics Based on spawn](./accepted/024-concurrency-model.md)                                        | Chen Xu   | 2026-06-05    | Accepted (Revised) |
| RFC-026    | [RFC-026: FFI Core Mechanism](./accepted/026-ffi-core-mechanism.md)                                                                 | Chen Xu   | 2026-07-03    | Accepted           |
| ↳ RFC-026a | [RFC-026a: Extensible FFI Mechanism System](./review/026a-extensible-ffi-system.md)                                                 | Chen Xu   | 2026-06-05    | Under Review       |
| ↳ RFC-026b | [RFC-026b: yx-bindgen Toolchain](./draft/026b-yx-bindgen.md)                                                                        | Chen Xu   | 2026-06-05    | Draft              |
| RFC-027    | [RFC-027: Compile-Time Predicates and Unified Static Verification](./accepted/027-compile-time-evaluation-types.md)                 | Chen Xu   | 2026-06-07    | Accepted           |
| RFC-029    | [RFC-029: Module Semantics System](./accepted/029-module-semantics.md)                                                              | Chen Xu   | 2026-06-13    | Accepted           |
| RFC-030    | [RFC-030: assert Assertion Mechanism](./accepted/030-assert-mechanism.md)                                                           | Chen Xu   | 2026-06-15    | Accepted           |
| RFC-036    | [RFC-036: std.test Testing Framework and yaoxiang test Command](./accepted/036-test-framework.md)                                   | Chen Xu   | 2026-07-26    | Accepted           |
| RFC-037    | [RFC-037: Industrial Distribution Plan — Compiler/Toolchain Packaging Based on cargo-dist](./accepted/037-industrial-packaging.md)  | ChenXu233 | 2026-07-26    | Accepted           |
| RFC-038    | [RFC-038: Statement Termination & Newline Rules](./accepted/038-statement-termination.md)                                           | ChenXu233 | 2026-08-05    | Accepted           |
| RFC-029f   | [RFC-029f: Compilation Target Role and Import Surface Semantics](./accepted/029f-target-semantics.md)                               | Chen Xu   | 2026-09-12    | Accepted           |

---

## Deprecated RFC

| Number  | Title                                                                                                                                                       | Author  | Creation Date | Status                           |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ------------- | -------------------------------- |
| RFC-001 | [RFC-001: Concurrent Model and Error Handling System](./deprecated/001-concurrent-model-error-handling.md)                                                  | Chen Xu | 2025-01-05    | Deprecated (Replaced by RFC-024) |
| RFC-020 | [RFC-020: Dynamic Modules and FFI Integration](./deprecated/020-dynamic-modules-ffi.md)                                                                     | Chen Xu | 2026-03-14    | Deprecated                       |
| RFC-021 | [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](./deprecated/021-library-driven-ffi-extension.md)                                   | Chen Xu | 2026-03-14    | Deprecated                       |
| RFC-022 | [RFC 022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](./deprecated/022-hoare-logic-static-verification.md) | Chen Xu | 2026-03-16    | Deprecated (Replaced by RFC-027) |
| RFC-023 | [RFC-023: Closure Capture Model](./deprecated/023-closure-capture-model.md)                                                                                 | Chen Xu | 2026-05-29    | Deprecated                       |

---

## Rejected RFC

| Number  | Title                                                                                                     | Author  | Creation Date | Status   |
| ------- | --------------------------------------------------------------------------------------------------------- | ------- | ------------- | -------- |
| RFC-003 | [RFC-003: Version Planning](./rejected/003-version-planning.md)                                           | Chen Xu | 2025-01-05    | Rejected |
| RFC-005 | [RFC-005: Automated CVE Security Scanning System](./rejected/005-automated-cve-scanning.md)               | Chen Xu | 2025-01-05    | Rejected |
| RFC-016 | [RFC 016: Quantum Native Support and Multi-Backend Integration](./rejected/016-quantum-native-support.md) | Chen Xu | 2026-02-13    | Rejected |
| RFC-025 | [RFC-025: Extensible Primitive Type Mechanism](./rejected/025-primitive-extension.md)                     | Chen Xu | 2026-06-05    | Rejected |

---

## RFC Lifecycle

```
Draft → Under Review → Accepted → Deprecated (Replaced)
                  ↓
               Rejected (Not Approved)
```

### Status Description

| Status           | Location          | Description                                                 |
| ---------------- | ----------------- | ----------------------------------------------------------- |
| **Draft**        | `rfc/draft/`      | Author's draft, awaiting submission for review              |
| **Under Review** | `rfc/review/`     | Open community discussion and feedback                      |
| **Accepted**     | `rfc/accepted/`   | Becomes formal design document, enters implementation phase |
| **Deprecated**   | `rfc/deprecated/` | Previously accepted, replaced by new design                 |
| **Rejected**     | `rfc/rejected/`   | Rejected RFC documents                                      |

---

## Document Revision Rules

**RFC documents must contain only correct information.** When making design changes, directly modify
the original text to express the current correct semantics; **Do not retain incorrect content and
then add an "errata" block to correct it**.

Retaining "original text + errata" is the worst approach: readers only discover halfway through that
everything before is void, the reading effort is wasted, and it's easy to mistakenly treat
deprecated sections as current semantic references. Errata blocks may seem cautious, but they
actually pass the organizing cost on to readers.

### Correct Approach

| Situation                                                                    | Approach                                                                                                                                        |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Implementation does not match the original text, original text is overturned | Directly rewrite the paragraph with correct content, delete the original statement                                                              |
| Example code no longer runs                                                  | Directly change to a runnable form; do not retain the old example                                                                               |
| Need to let readers know "what it used to look like"                         | Only keep incorrect content when **specifically making an error comparison**, and label it adjacent as "this writing is incorrect" with reasons |
| Need to trace design evolution                                               | Write in Git commit messages or issues, not in the RFC body                                                                                     |

### Exceptions

The following error information may be retained:

- **Specifically making comparison teaching**: clearly marked as correct/incorrect comparison, and
  the incorrect side is adjacent with explanation "why it's wrong"
- **Deprecated RFC** (`rfc/deprecated/`): as historical record, but should be marked as to who
  replaced it

### Auxiliary Measures

- The `status` / `updated` fields at the top of the RFC reflect the latest revision time; no need to
  write "what was revised this time" in the body
- Implementation status is expressed with ✅ / ❌ in tables; avoid interleaving status narratives in
  the body
- The complete revision history is viewed with `git log -- <file>`

---

## Submitting RFC

1. Read [RFC_TEMPLATE.md](RFC_TEMPLATE.md) to understand the format requirements
2. Reference [EXAMPLE_full_feature_proposal.md](EXAMPLE_full_feature_proposal.md) to learn the
   writing style
3. Create a new file named `number-descriptive-title.md`
4. Place the file in the `docs/reference/rfc/draft/` directory
5. Update this index file to add the new RFC entry
6. Submit a PR to enter the review process

---

## Contributing Guide

Please refer to CONTRIBUTING.md for the contributing guide.
