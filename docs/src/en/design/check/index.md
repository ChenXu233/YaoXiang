---
title: 'check Command Design Document'
description: 'Design specification for the yaoxiang check static checking tool'
---

# check Command Design Document

`yaoxiang check` is the YaoXiang compiler's static checking tool, providing type checking,
cross-file analysis, and incremental checking capabilities.

## Design Principles

1. **Zero false positives**: Every error reported must be a real error
2. **Cross-file awareness**: Correctly detect type errors and undefined references across modules
3. **Incremental first**: In watch mode, only affected files are re-checked
4. **Self-documenting**: Error codes, message templates, and help text are all managed through i18n

## Document Navigation

- [Diagnostic System](./diagnostic-system.md) — Error code system, Diagnostic data structure,
  Emitter output
- [Cross-file Analysis](./cross-file-analysis.md) — Shared type environment, dependency graph,
  topological sort
- [Incremental Checking](./incremental-checking.md) — CheckSession, affected_modules, watch mode

## Boundaries with Other Systems

| System                      | Responsibilities                                                 | Relationship with check             |
| --------------------------- | ---------------------------------------------------------------- | ----------------------------------- |
| Compiler (`yaoxiang build`) | Complete compilation (parsing → type checking → code generation) | check only does the first two steps |
| LSP                         | Editor integration (completion, navigation, diagnostics)         | check's diagnostics are reusable    |
| Formatter (`yaoxiang fmt`)  | Code style                                                       | Independent, used in parallel in CI |
