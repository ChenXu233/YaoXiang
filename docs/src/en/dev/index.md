---
title: 'Development Documentation'
description:
  'Entry point for contributors and maintainers: modify code, modify documentation, architecture
  docs, tool design specs, process specs'
---

# Development Documentation

For contributors and maintainers. All changes enter through the same handbook, and each type of
change follows its own set of rules.

## Before You Start (Required Reading)

| What you want to do | Entry point (unified self-check) | Rules proper                         |
| ------------------- | -------------------------------- | ------------------------------------ |
| Modify code         | [HOWTO.md](./HOWTO.md) §1        | [coding-rules.md](./coding-rules.md) |
| Modify docs         | [HOWTO.md](./HOWTO.md) §2        | [docs-rules.md](./docs-rules.md)     |

Not sure which category you fall into? Read step 0 of HOWTO first; it will determine for you.

## Compiler Architecture

[architecture/](./architecture/) contains the supporting design documents for the RFC-039
refactoring: 01 Feature Routing, 02 Phase Contracts, 03 Type Representation, 04 SSA, 05 Frontend
Paradigm, 06 Cleanup Checklist, 07 Equivalence Criteria, 08 Maintenance Mechanism, 09 Work Breakdown
Structure (WBS).

## Tool Design Specifications

- [design/check/](./design/check/): Design specification for the `yx check` static analysis (zero
  false-positive principle, cross-file analysis, incremental checking)
- [design/formatter/](./design/formatter/): Behavior specification for the `yx format` formatter

> User-facing command usage is in the reference directory: [check](../reference/check-command.md),
> [format](../reference/format-command.md), [test](../reference/test-command.md).

## Process and Specifications

- [Contributing Guide](./contributing.md): How to participate in development
- [Commit Convention](./commit-convention.md): Git commit message format
- [Branch Maintenance Guide](./branch-maintenance-guide.md): Branch management strategy
- [Release Process](./release.md): Version release and artifact distribution
- [Testing Specification](./test-specification.md): Test layering, corpus organization, and gating

## What Is Not Included Here

- Language feature proposals and decisions → [rfc/](../rfc/)
- Language philosophy and manifestos → [explanation/](../explanation/)
- User documentation for commands → [reference/](../reference/)
