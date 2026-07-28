---
title: check Command Design Document
description: Design specification for yaoxiang check static analysis tool
---

# check Command Design Document

`yaoxiang check` is the static analysis tool for the YaoXiang compiler, providing type checking, cross-file analysis, and incremental checking capabilities.

## Design Principles

1. **Zero False Positives**: Every reported error must be a genuine error
2. **Cross-File Awareness**: Correctly detect cross-module type errors and undefined references
3. **Incrementality First**: Watch mode only re-checks affected files
4. **Self-Documenting**: Error codes, message templates, and help text are all managed through i18n

## Document Navigation

- [Diagnostic System](./diagnostic-system.md) — Error code system, Diagnostic data structure, Emitter output
- [Cross-File Analysis](./cross-file-analysis.md) — Shared type environment, dependency graph, topological sort
- [Incremental Checking](./incremental-checking.md) — CheckSession, affected_modules, watch mode

## Boundaries with Other Systems

| System                      | Responsibilities                                      | Relationship with check    |
| --------------------------- | ----------------------------------------------------- | -------------------------- |
| Compiler (`yaoxiang build`) | Full compilation (parsing → type checking → codegen) | check only does first two steps |
| LSP                         | Editor integration (completions, go-to, diagnostics) | check diagnostics are reusable |
| Formatter (`yaoxiang fmt`)  | Code style                                            | Independent, runs in parallel in CI |
