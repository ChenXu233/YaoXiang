---
title: "RFC Implementation Status"
---

# RFC Implementation Status

> **Last Updated**: 2026-06-05
> **Scope of Analysis**: 14 Accepted RFCs

---

## Overview

| RFC | Title | Status | Remaining Key Steps |
|-----|-------|--------|---------------------|
| RFC-001 | Concurrency Model and Error Handling | Abandoned | Superseded by RFC-024 |
| RFC-004 | Multi-Position Union Binding for Curried Methods | In Progress | [positions] syntax, multi-position union binding, auto-currying, default binding |
| RFC-006 | Documentation Site Construction | Near Completion | Version switcher menu |
| RFC-007 | Function Definition Syntax Unification | Near Completion | yaoxiang-migrate migration tool |
| RFC-008 | Runtime Concurrency Model Decoupled from Scheduler | Phase A Complete | Compile-time DAG analysis (covered by RFC-024), LLVM AOT backend |
| RFC-009 | Ownership Model | Near Completion | Brand mechanism, ref escape analysis |
| RFC-010 | Unified Type Syntax | Near Completion | Duck typing support, interface composition (intersection types) |
| RFC-011 | Generics System Design | In Progress | Value-dependent types, compile-time evaluation engine, decreases clause, conditional types, type families |
| RFC-012 | F-String Template Strings | Near Completion | Format specifiers (`:.2f`, etc.) |
| RFC-013 | Error Code Specification Design | Near Completion | yaoxiang explain CLI command |
| RFC-014 | Package Management System Design | Near Completion | Registry source, workspace support, dependency overrides |
| RFC-015 | Configuration System Design | In Progress | User-level config, config merging, yaoxiang config CLI, CLI/env var overrides |
| RFC-017 | LSP Language Server Support | Near Completion | Incremental sync, TCP/Unix Socket, DAP debugger adapter |
| RFC-023 | Closure Capture Model | Near Completion | Complete escape analysis |
| RFC-024 | Concurrency Model Based on spawn Blocks | In Progress | Old model cleanup, compile-time DAG analysis, runtime integration, spawn for |

---

## Near-Completion RFCs (8)

Light remaining workload, each RFC only missing 1-2 features.

### RFC-006: Documentation Site Construction

**Completed**: VitePress site, multi-language support (zh, en, ja, ru), CI/CD auto-deployment, sidebar/navbar configuration, search functionality

**Remaining**:
- ❌ Version switcher menu (/v0.5/zh/ format)

### RFC-007: Function Definition Syntax Unification

**Completed**: `name: (params) -> Return = body` syntax parsing, lambda expression syntax, HM type inference, function definition syntax unification

**Remaining**:
- ❌ yaoxiang-migrate migration tool

### RFC-012: F-String Template Strings

**Completed**: `f"Hello {name}"` prefix syntax, variable interpolation/expression evaluation, compile-time conversion to string operations

**Remaining**:
- ❌ Full support for format specifiers (`:.2f`, etc.)

### RFC-013: Error Code Specification Design

**Completed**: Four-digit numbering `Exxxx`, multi-language resource files (JSON i18n), `DiagnosticBuilder` generic builder, `error!` macro

**Remaining**:
- ⚠️ yaoxiang explain CLI command pending confirmation

### RFC-023: Closure Capture Model

**Completed**: Compiler auto-analysis of external variables referenced in closure bodies, Dup type direct copy, zero annotations

**Remaining**:
- ⚠️ Complete escape analysis pending refinement

### RFC-009: Ownership Model

**Completed**: Move semantics (default), &T/&mut T borrow tokens, clone() explicit deep copy, unsafe + *T, cross-task cycle detection, Send/Sync constraints, freeze mechanism, token conflict detection

**Remaining**:
- ❌ Brand mechanism (compiler-internal unique token identifiers)
- ❌ ref auto-selection of Rc/Arc escape analysis

### RFC-010: Unified Type Syntax

**Completed**: Unified declaration syntax `name: type = value`, record type definitions, interface definition and implementation checking, method definition syntax (Type.method), generic call `()` syntax, structural subtyping check

**Remaining**:
- ❌ Full duck typing support
- ❌ Interface composition (`Drawable & Serializable` intersection types)

### RFC-017: LSP Language Server Support

**Completed**: Standalone LSP server process, JSON-RPC communication, code completion/definition goto/diagnostics/reference search/hover, ghost text (Inlay Hints), semantic tokens, rename, code actions

**Remaining**:
- ❌ Incremental sync
- ❌ TCP/Unix Socket remote communication
- ❌ DAP debugger adapter
- ❌ Ownership semantics visualization

---

## In-Progress RFCs (4)

Core functionality started, but still significant unfinished work.

### RFC-001: Concurrency Model and Error Handling (Abandoned)

**Superseded by RFC-024**. Old `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` trait, whole-program DAG analysis scheme are abandoned. New concurrency model see RFC-024.

### RFC-004: Multi-Position Union Binding for Curried Methods

**Completed**: Basic method binding syntax (Type.method definition), basic method call syntax sugar

**Remaining**:
- ❌ `[positions]` position index binding syntax (`[0]`, `[-1]`, etc.)
- ❌ Multi-position union binding `[0, 1]`
- ❌ Auto-currying binding
- ❌ Default binding logic

### RFC-011: Generics System Design

**Completed**: Unified signature syntax `(T: Type, R: Type) -> ...`, Type self-describing mechanism, type constraints `T: Dup + Add`, associated types (GAT), function overloading specialization, monomorphization, dead code elimination

**Remaining**:
- ❌ Value-dependent types (`Vec: (n: Int) -> Type`)
- ❌ Compile-time evaluation engine (type-position function call compile-time evaluation)
- ❌ decreases clause (termination check)
- ❌ Conditional types `If: (C: Bool, T: Type, E: Type) -> Type`
- ❌ Type families

### RFC-015: Configuration System Design

**Completed**: Basic configuration module, project-level `yaoxiang.toml` parsing

**Remaining**:
- ❌ User-level config `~/.config/yaoxiang/config.toml`
- ❌ Configuration merging logic
- ❌ `yaoxiang config` CLI command
- ❌ CLI/env var overrides
- ❌ `[tool.*]` extensions
- ❌ `platform` platform constraints

---

## Not Started RFCs (1)

### RFC-008: Runtime Concurrency Model Decoupled from Scheduler (Phase A Complete)

**Completed**: Embedded / Standard / Full three-tier runtime, DAG scheduler, cooperative time slicing, resource serialization, failure/cancel propagation

**Remaining**:
- ❌ Compile-time DAG analysis (covered by RFC-024)
- ❌ LLVM AOT backend
- ❌ Generic scheduler interface
- ❌ Scheduler static library linking

---

## Implementation Checklist (by Priority)

### High Priority (Core Language Features)

1. **Old concurrency model cleanup** (RFC-024) — Remove `@block`/`@eager`/`@auto`, `EvalMode`, `Send`/`Sync`
2. **Compile-time DAG analysis** (RFC-024) — spawn block dependency analysis, topological sort, execution plan generation
3. **Runtime integration** (RFC-024) — Interpreter grouped parallel execution per execution plan
4. **Value-dependent types** (RFC-011) — `Vec: (n: Int) -> Type`
5. **decreases clause/termination check** (RFC-011) — Compile-time evaluation safety guarantee
6. **Conditional types** (RFC-011) — `If: (C: Bool, T: Type, E: Type) -> Type`
7. **`[positions]` position index binding syntax** (RFC-004)
8. **LLVM AOT backend** (RFC-008)
9. **Generic scheduler interface** (RFC-008)

### Medium Priority (Toolchain)

9. **User-level config** (RFC-015) — `~/.config/yaoxiang/config.toml`
10. **Configuration merging logic** (RFC-015)
11. **`yaoxiang config` CLI command** (RFC-015)
12. **`yaoxiang explain` CLI command** (RFC-013)
13. **`yaoxiang-migrate` migration tool** (RFC-007)
14. **Registry source** (RFC-014)
15. **Integrity verification** (RFC-014)

### Low Priority (Enhancement Features)

16. **Format specifiers** (RFC-012) — `f"Pi: {pi:.2f}"`
17. **Version switcher menu** (RFC-006)
18. **Interface composition** (RFC-010) — `Drawable & Serializable`
19. **TCP/Unix Socket remote communication** (RFC-017)
20. **DAP debugger adapter** (RFC-017)
21. **Ownership semantics visualization** (RFC-017)
22. **spawn for data-parallel loop** (RFC-024)
23. **Resource type system** (RFC-024)
24. **`[tool.*]` third-party tool config extensions** (RFC-015)
25. **`platform` platform constraints** (RFC-015)