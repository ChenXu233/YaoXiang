---
title: 'check Command Design Document'
description: 'Design specification for the yx check static checking tool'
---

# check Command Design Document

`yx check` is the YaoXiang compiler's static checking tool, providing type checking, cross-file
analysis, and incremental checking functionality.

## Design Principles

1. **Zero false positives**: Every reported error must be a real error
2. **Cross-file awareness**: Correctly detect cross-module type errors and undefined references
3. **Incremental first**: watch mode only re-checks affected files
4. **Self-documenting**: Error codes, message templates, and help text are all managed through i18n

## Document Navigation

- [Diagnostic System](./diagnostic-system.md) — Error code system, Diagnostic data structure,
  Emitter output
- [Cross-file Analysis](./cross-file-analysis.md) — Shared type environment, dependency graph,
  topological sort
- [Incremental Checking](./incremental-checking.md) — CheckSession, affected_modules, watch mode

## Boundaries with Other Systems

| System                  | Responsibility                                              | Relationship with check             |
| ----------------------- | ----------------------------------------------------------- | ----------------------------------- |
| Compiler (`yx build`)   | Complete compilation (parse → type check → code generation) | check only does the first two steps |
| LSP                     | Editor integration (completion, jump, diagnostics)          | check's diagnostics are reusable    |
| Formatter (`yx format`) | Code style                                                  | Independent, used in parallel in CI |
