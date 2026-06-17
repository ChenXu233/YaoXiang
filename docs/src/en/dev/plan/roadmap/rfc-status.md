---
title: "RFC Implementation Status"
---

# RFC Implementation Status

> **Last Updated**: 2026-06-05
> **Scope of Analysis**: 14 Accepted RFCs

---

## Overview

| RFC | Title | Status | Remaining Key Steps |
|-----|------|------|---------------------|
| RFC-001 | spawn Model and Error Handling | Deprecated | Replaced by RFC-024 |
| RFC-004 | Multi-Position Union Binding of Curried Methods | In Progress | `[positions]` syntax, multi-position union binding, automatic currying, default binding |
| RFC-006 | Documentation Site Construction and Optimization | Near Completion | Version switcher menu |
| RFC-007 | Unified Function Definition Syntax | Near Completion | `yaoxiang-migrate` migration tool |
| RFC-008 | Runtime Concurrency Model and Scheduler Decoupling | Stage A Complete | compile-time DAG analysis (covered by RFC-024), LLVM AOT backend |
| RFC-009 | Ownership Model | Near Completion | branding mechanism, ref escape analysis |
| RFC-010 | Unified Type Syntax | Near Completion | duck typing support, interface composition (intersection types) |
| RFC-011 | Generics System Design | In Progress | value-dependent types, compile-time evaluation engine, decreases reduction, conditional types, type families |
| RFC-012 | F-String Template Strings | Near Completion | format specifiers (`:.2f` etc.) |
| RFC-013 | Error Code Specification Design | Near Completion | `yaoxiang explain` CLI command |
| RFC-014 | Package Management System Design | Near Completion | Registry sources, workspace support, dependency overrides |
| RFC-015 | Configuration System Design | In Progress | user-level configuration, configuration merging, `yaoxiang config` CLI, command-line/environment variable overrides |
| RFC-017 | LSP Language Server Support | Near Completion | incremental sync, TCP/Unix Socket, DAP debug adapter |
| RFC-023 | Closure Capture Model | Near Completion | complete escape analysis |
| RFC-024 | Concurrency Model Based on `spawn` Blocks | In Progress | clean up old model, compile-time DAG analysis, runtime integration, `spawn for` |

---

## RFCs Near Completion (8)

Small remaining workload; each RFC is missing only 1–2 features.

### RFC-006: Documentation Site Construction

**Completed**: VitePress site, multilingual support (zh, en, ja, ru), CI/CD auto-deployment, sidebar/navigation bar configuration, search functionality

**Remaining**:
- ❌ Version switcher menu (`/v0.5/zh/` format)

### RFC-007: Unified Function Definition Syntax

**Completed**: `name: (params) -> Return = body` syntax parsing, Lambda expression syntax, HM type inference, unified function definition syntax

**Remaining**:
- ❌ `yaoxiang-migrate` migration tool

### RFC-012: F-String Template Strings

**Completed**: `f"Hello {name}"` prefix syntax, variable interpolation / expression evaluation, compile-time conversion to string operations

**Remaining**:
- ❌ Complete support for format specifiers (`:.2f` etc.)

### RFC-013: Error Code Specification Design

**Completed**: Four-digit numbering `Exxxx`, multilingual resource files (JSON i18n), generic `DiagnosticBuilder`, `error!` macro

**Remaining**:
- ⚠️ `yaoxiang explain` CLI command pending confirmation

### RFC-023: Closure Capture Model

**Completed**: Compiler automatically analyzes external variables referenced by the closure body, direct copy for `Dup` types, zero annotations

**Remaining**:
- ⚠️ Complete escape analysis to be polished

### RFC-009: Ownership Model

**Completed**: Move semantics (default), `&T` / `&mut T` borrow tokens, explicit `clone()` deep copy, `unsafe` + `*T`, cross-task cycle detection, `Send` / `Sync` constraints, token conflict detection

**Remaining**:
- ❌ Branding mechanism (unique identifier for compiler-internal tokens)
- ❌ `ref` automatic escape analysis to choose `Rc` / `Arc`

### RFC-010: Unified Type Syntax

**Completed**: Unified declaration syntax `name: type = value`, record type definitions, interface definition and implementation checks, method definition syntax (`Type.method`), generic call `()` syntax, structural subtyping check

**Remaining**:
- ❌ Complete duck typing support
- ❌ Interface composition (`Drawable & Serializable` intersection types)

### RFC-017: LSP Language Server Support

**Completed**: Standalone LSP server process, JSON-RPC communication, code completion / go to definition / diagnostics / find references / hover hints, Inlay Hints, Semantic Tokens, rename, code actions

**Remaining**:
- ❌ Incremental synchronization
- ❌ TCP / Unix Socket remote communication
- ❌ DAP debug adapter
- ❌ Ownership semantics visualization

---

## RFCs In Progress (4)

Core functionality has been started, but significant work remains.

### RFC-001: spawn Model and Error Handling (Deprecated)

**Replaced by RFC-024**. The old `@block` / `@eager` / `@auto` annotations, `Send` / `Sync` traits, and whole-program DAG analysis scheme have been deprecated. See RFC-024 for the new concurrency model.

### RFC-004: Multi-Position Union Binding of Curried Methods

**Completed**: Basic method binding syntax (`Type.method` definition), basic method call syntactic sugar

**Remaining**:
- ❌ `[positions]` positional index binding syntax (`[0]`, `[-1]`, etc.)
- ❌ Multi-position union binding `[0, 1]`
- ❌ Automatic currying binding
- ❌ Default binding logic

### RFC-011: Generics System Design

**Completed**: Unified signature syntax `(T: Type, R: Type) -> ...`, `Type` self-describing mechanism, type constraints `T: Dup + Add`, associated types (GAT), function overload specialization, monomorphization, dead code elimination

**Remaining**:
- ❌ Value-dependent types (`Vec: (n: Int) -> Type`)
- ❌ Compile-time evaluation engine (compile-time evaluation of function calls at type position)
- ❌ `decreases` reduction (termination check)
- ❌ Conditional type `If: (C: Bool, T: Type, E: Type) -> Type`
- ❌ Type families

### RFC-015: Configuration System Design

**Completed**: Basic configuration module, project-level `yaoxiang.toml` parsing

**Remaining**:
- ❌ User-level configuration `~/.config/yaoxiang/config.toml`
- ❌ Configuration merging logic
- ❌ `yaoxiang config` CLI command
- ❌ Command-line / environment variable overrides
- ❌ `[tool.*]` extensions
- ❌ `platform` constraint

---

## RFCs Not Started (1)

### RFC-008: Runtime Concurrency Model and Scheduler Decoupling (Stage A Complete)

**Completed**: Embedded / Standard / Full three-tier runtime, DAG scheduler, cooperative time slicing, resource serialization, failure / cancellation propagation

**Remaining**:
- ❌ compile-time DAG analysis (covered by RFC-024)
- ❌ LLVM AOT backend
- ❌ Generic scheduler interface
- ❌ Scheduler static library linking

---

## Unimplemented Checklist (Sorted by Priority)

### High Priority (Core Language Features)

1. ~~Clean up old concurrency model~~ (RFC-024) — ✅ Done: `@block` / `@eager` / `@auto`, `Send` / `Sync` have been removed from source code and documentation
2. **Compile-time DAG analysis** (RFC-024) — dependency analysis inside `spawn` blocks, topological sort, execution plan generation
3. **Runtime integration** (RFC-024) — interpreter groups execution plans for parallel execution
4. **Value-dependent types** (RFC-011) — `Vec: (n: Int) -> Type`
5. **`decreases` reduction / termination check** (RFC-011) — compile-time evaluation safety guarantee
6. **Conditional types** (RFC-011) — `If: (C: Bool, T: Type, E: Type) -> Type`
7. **`[positions]` positional index binding syntax** (RFC-004)
8. **LLVM AOT backend** (RFC-008)
9. **Generic scheduler interface** (RFC-008)

### Medium Priority (Toolchain)

9. **User-level configuration** (RFC-015) — `~/.config/yaoxiang/config.toml`
10. **Configuration merging logic** (RFC-015)
11. **`yaoxiang config` CLI command** (RFC-015)
12. **`yaoxiang explain` CLI command** (RFC-013)
13. **`yaoxiang-migrate` migration tool** (RFC-007)
14. **Registry sources** (RFC-014)
15. **Integrity verification** (RFC-014)

### Low Priority (Enhancement Features)

16. **Format specifiers** (RFC-012) — `f"Pi: {pi:.2f}"`
17. **Version switcher menu** (RFC-006)
18. **Interface composition** (RFC-010) — `Drawable & Serializable`
19. **TCP / Unix Socket remote communication** (RFC-017)
20. **DAP debug adapter** (RFC-017)
21. **Ownership semantics visualization** (RFC-017)
22. **`spawn for` data-parallel loop** (RFC-024)
23. **Resource type system** (RFC-024)
24. **`[tool.*]` third-party tool configuration extensions** (RFC-015)
25. **`platform` constraint** (RFC-015)