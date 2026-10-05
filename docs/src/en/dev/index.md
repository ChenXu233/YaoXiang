---
title: 'Development Documentation'
description:
  'Contributor and maintainer hub: code and docs handbooks, architecture, tool design specs, process
  conventions'
---

# Development Documentation

For contributors and maintainers. Every change enters through one handbook, then follows the
rulebook that matches its type.

## Before You Start (Required)

| You want to | Entry (unified self-check)             | Rulebook                                          |
| ----------- | -------------------------------------- | ------------------------------------------------- |
| Change code | [HOWTO.md](../../dev/HOWTO.md) §1 (zh) | [coding-rules.md](../../dev/coding-rules.md) (zh) |
| Change docs | [HOWTO.md](../../dev/HOWTO.md) §2 (zh) | [docs-rules.md](../../dev/docs-rules.md) (zh)     |

> The handbook and rulebooks are not yet translated; the links above point to the Chinese originals
> until the translation workflow catches up. If you are unsure which row you are, read step 0 of the
> HOWTO — it decides for you.

## Compiler Architecture

[architecture/](./architecture/) holds the companion design documents of the RFC-039 refactoring: 01
routing, 02 stage contracts, 03 type representation, 04 SSA, 05 frontend paradigm, 06 cleanup
inventory, 07 equivalence oracle, 08 maintenance mechanism, 09 execution WBS.

## Tool Design Specifications

- [design/check/](./design/check/): the design specification of yx check (zero false positives,
  cross-file analysis, incremental checking)
- [design/formatter/](./design/formatter/): the behavior specification of the yx format tool

> The user-facing command references live under [reference/](../reference/): check, format, test.

## Process & Conventions

- [Contributing Guide](./contributing.md): how to participate
- [Commit Convention](./commit-convention.md): Git commit message format
- [Branch Maintenance Guide](./branch-maintenance-guide.md): branch management strategy
- [Release Process](./release.md): version release and artifact distribution
- [Test Specification](./test-specification.md): test layering, corpus organization, CI gates

## What Does Not Belong Here

- Language feature proposals and decisions → [rfc/](../rfc/)
- Language philosophy and manifestos → [explanation/](../explanation/)
- User documentation of commands → [reference/](../reference/)
