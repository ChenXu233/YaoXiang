---
title: check Command Design Document
description: Design specification for the yaoxiang check static analysis tool
---

# check Command Design Document

`yaoxiang check` is YaoXiang's static analysis tool, providing type checking, cross-file analysis, and incremental checking capabilities.

## Design Principles

1. **Zero false positives**: Every reported error must be a real error
2. **Cross-file aware**: Correctly detect cross-module type errors and undefined references
3. **Incremental first**: Watch mode only re-checks affected files
4. **Self-documenting**: Error codes, message templates, and help text are all managed via i18n

## Document Navigation

- [Diagnostic System](./diagnostic-system.md) — Error code system, Diagnostic data structure, Emitter output
- [Cross-file Analysis](./cross-file-analysis.md) — Shared type environment, dependency graph, topological sort
- [Incremental Checking](./incremental-checking.md) — CheckSession, affected_modules, watch mode

## Boundaries with Other Systems

| System | Responsibility | Relationship with check |
|--------|----------------|-------------------------|
| Compiler (`yaoxiang build`) | Full compilation (parsing → type checking → code generation) | check only does the first two steps |
| LSP | Editor integration (completion, go-to-definition, diagnostics) | check's diagnostics are reusable |
| Formatter (`yaoxiang fmt`) | Code style | Independent, used in parallel in CI |