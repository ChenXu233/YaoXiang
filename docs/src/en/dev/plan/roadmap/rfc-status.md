---
title: "RFC Implementation Status"
---

# RFC Implementation Status

> **Last Updated**: 2026-06-05
> **Scope of Analysis**: 14 accepted RFCs

---

## Overview

| RFC | Title | Status | Remaining Key Steps |
|-----|-------|--------|---------------------|
| RFC-001 | Spawn Model and Error Handling | Deprecated | Superseded by RFC-024 |
| RFC-004 | Multi-position Union Binding for Curried Methods | In Progress | [positions] syntax, multi-position union binding, automatic currying, default binding |
| RFC-006 | Documentation Site Construction | Near Completion | Version switch menu |
| RFC-007 | Unified Function Definition Syntax | Near Completion | yaoxiang-migrate migration tool |
| RFC-008 | Runtime Concurrency Model and Scheduler Decoupling | Phase A Complete | Compile-time DAG analysis (covered by RFC-024), LLVM AOT backend |
| RFC-009 | Ownership Model | Near Completion | Brand mechanism, ref escape analysis |
| RFC-010 | Unified Type Syntax | Near Completion | Full duck typing support, interface composition (intersection types) |
| RFC-011 | Generics System Design | In Progress | Value-dependent types, compile-time evaluation engine, decreases clause, conditional types, type families |
| RFC-012 | F-String Template Strings | Near Completion | Format specifiers (`:.2f` etc.) |
| RFC-013 | Error Code Specification Design | Near Completion | yaoxiang explain CLI command |
| RFC-014 | Package Management System Design | Near Completion | Registry source, workspace support, dependency override |
| RFC-015 | Configuration System Design | In Progress | User-level configuration, config merge, yaoxiang config CLI, command-line/environment variable override |
| RFC-017 | LSP Language Server Support | Near Completion | Incremental sync, TCP/Unix Socket, DAP debugger adapter |
| RFC-023 | Closure Capture Model | Near Completion | Complete escape analysis |
| RFC-024 | Concurrency Model Based on Spawn Blocks | In Progress | Cleanup old model, compile-time DAG analysis, runtime integration, spawn for |

---

## Near Completion RFCs (8)

Minimal remaining work, each RFC missing only 1-2 features.

### RFC-006: Documentation Site Construction

**Completed**: VitePress site, multi-language support (zh, en, ja, ru), CI/CD auto-deployment, sidebar/navigation config, search functionality

**Remaining**:
- ❌ Version switch menu (/v0.5/zh/ format)

### RFC-007: Unified Function Definition Syntax

**Completed**: `name: (params) -> Return = body` syntax parsing, lambda expression syntax, HM type inference, function definition syntax unification

**Remaining**:
- ❌ yaoxiang-migrate migration tool

### RFC-012: F-String Template Strings

**Completed**: `f"Hello {name}"` prefix syntax, variable interpolation/expression evaluation, compile-time conversion to string operations

**Remaining**:
- ❌ Full support for format specifiers (`:.2f` etc.)

### RFC-013: Error Code Specification Design

**Completed**: Four-digit numbering `Exxxx`, multi-language resource files (JSON i18n), `DiagnosticBuilder` common builder, `error!` macro

**Remaining**:
- ⚠️ yaoxiang explain CLI command pending confirmation

### RFC-023: Closure Capture Model

**Completed**: Compiler automatic analysis of external variables referenced in closure body, Dup type direct copy, zero annotations

**Remaining**:
- ⚠️ Complete escape analysis pending improvement

### RFC-009: Ownership Model

**Completed**: Move semantics (default), &T/&mut T borrow tokens, clone() explicit deep copy, unsafe + *T, cross-task cycle detection, Send/Sync constraints, token conflict detection

**Remaining**:
- ❌ Brand mechanism (compiler internal token unique identifier)
- ❌ Ref automatic selection of Rc/Arc escape analysis

### RFC-010: Unified Type Syntax

**Completed**: Unified declaration syntax `name: type = value`, record type definition, interface definition and implementation checking, method definition syntax (Type.method), generic call `()` syntax, structural subtyping checks

**Remaining**:
- ❌ Full duck typing support
- ❌ Interface composition (`Drawable & Serializable` intersection types)

### RFC-017: LSP Language Server Support

**Completed**: Standalone LSP server process, JSON-RPC communication, code completion/definition jump/diagnostics/reference search/hover info, inlay hints, semantic tokens, rename, code actions

**Remaining**:
- ❌ Incremental sync
- ❌ TCP/Unix Socket remote communication
- ❌ DAP debugger adapter
- ❌ Ownership semantics visualization

---

## In Progress RFCs (4)

Core functionality started, but substantial work remains.

### RFC-001: Spawn Model and Error Handling (Deprecated)

**Superseded by RFC-024**. The old `@block`/`@eager`/`@auto` annotations, Send/Sync trait, and whole-program DAG analysis proposal are deprecated. For the new concurrency model, see RFC-024.

### RFC-004: Multi-position Union Binding for Curried Methods

**Completed**: Basic method binding syntax (Type.method definition), basic method call syntax sugar

**Remaining**:
- ❌ `[positions]` position index binding syntax (`[0]`, `[-1]`, etc.)
- ❌ Multi-position union binding `[0, 1]`
- ❌ Automatic currying binding
- ❌ Default binding logic

### RFC-011: Generics System Design

**Completed**: Unified signature syntax `(T: Type, R: Type) -> ...`, Type self-describing mechanism, type constraints `T: Dup + Add`, associated types (GAT), function overload specialization, monomorphization, dead code elimination

**Remaining**:
- ❌ Value-dependent types (`Vec: (n: Int) -> Type`)
- ❌ Compile-time evaluation engine (compile-time evaluation of functions at type positions)
- ❌ decreases clause (termination checking)
- ❌ Conditional types `If: (C: Bool, T: Type, E: Type) -> Type`
- ❌ Type families

### RFC-015: Configuration System Design

**Completed**: Basic configuration module, project-level `yaoxiang.toml` parsing

**Remaining**:
- ❌ User-level configuration `~/.config/yaoxiang/config.toml`
- ❌ Config merge logic
- ❌ yaoxiang config CLI command
- ❌ Command-line/environment variable override
- ❌ `[tool.*]` extension
- ❌ `platform` platform constraints

---

## Not Started RFCs (1)

### RFC-008: Runtime Concurrency Model and Scheduler Decoupling (Phase A Complete)

**Completed**: Embedded / Standard / Full three-tier runtime, DAG scheduler, cooperative time slicing, resource serialization, failure/cancellation propagation

**Remaining**:
- ❌ Compile-time DAG analysis (covered by RFC-024)
- ❌ LLVM AOT backend
- ❌ Generic scheduler interface
- ❌ Scheduler static library linking

---

## Unimplemented Items (By Priority)

### High Priority (Core Language Features)

1. **Cleanup old concurrency model** (RFC-024) — Remove `@block`/`@eager`/`@auto`, EvalMode, Send/Sync
2. **Compile-time DAG analysis** (RFC-024) — Dependency analysis within spawn blocks, topological sorting, execution plan generation
3. **Runtime integration** (RFC-024) — Interpreter executes groups in parallel according to execution plan
4. **Value-dependent types** (RFC-011) — `Vec: (n: Int) -> Type`
5. **decreases clause / termination checking** (RFC-011) — Compile-time evaluation safety guarantee
6. **Conditional types** (RFC-011) — `If: (C: Bool, T: Type, E: Type) -> Type`
7. **`[positions]` position index binding syntax** (RFC-004)
8. **LLVM AOT backend** (RFC-008)
9. **Generic scheduler interface** (RFC-008)

### Medium Priority (Tooling)

9. **User-level configuration** (RFC-015) — `~/.config/yaoxiang/config.toml`
10. **Config merge logic** (RFC-015)
11. **`yaoxiang config` CLI command** (RFC-015)
12. **`yaoxiang explain` CLI command** (RFC-013)
13. **`yaoxiang-migrate` migration tool** (RFC-007)
14. **Registry source** (RFC-014)
15. **Integrity verification** (RFC-014)

### Low Priority (Enhancement Features)

16. **Format specifiers** (RFC-012) — `f"Pi: {pi:.2f}"`
17. **Version switch menu** (RFC-006)
18. **Interface composition** (RFC-010) — `Drawable & Serializable`
19. **TCP/Unix Socket remote communication** (RFC-017)
20. **DAP debugger adapter** (RFC-017)
21. **Ownership semantics visualization** (RFC-017)
22. **spawn for data parallel loop** (RFC-024)
23. **Resource type system** (RFC-024)
24. **`[tool.*]` third-party tool config extension** (RFC-015)
25. **`platform` platform constraints** (RFC-015)