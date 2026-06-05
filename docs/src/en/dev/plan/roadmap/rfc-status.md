---
title: "RFC Implementation Status"
---

# RFC Implementation Status

> **Last Updated**: 2026-06-04
> **Scope of Analysis**: 14 Accepted RFCs

---

## Overview

| RFC | Title | Status | Remaining Key Steps |
|-----|-------|--------|---------------------|
| RFC-001 | Spawn Model and Error Handling | In Progress | DAG Dependency Analyzer, @block/@eager Full Execution, Resource Type System, Error Graph Visualization |
| RFC-004 | Multi-Position Union Binding for Curried Methods | In Progress | [positions] Syntax, Multi-Position Union Binding, Auto-Currying, Default Binding |
| RFC-006 | Documentation Site Construction | Near Completion | Version Switcher Menu |
| RFC-007 | Unified Function Definition Syntax | Near Completion | yaoxiang-migrate Migration Tool |
| RFC-008 | Runtime Concurrency Model and Scheduler Decoupling | Not Started | LLVM AOT Backend, Generic Scheduler Interface, Scheduler Static Library |
| RFC-009 | Ownership Model | Near Completion | Branding Mechanism, ref Escape Analysis |
| RFC-010 | Unified Type Syntax | Near Completion | Duck Typing Support, Interface Composition (Intersection Types) |
| RFC-011 | Generics System Design | In Progress | Value-Dependent Types, Compile-Time Evaluation Engine, decreases Clauses, Conditional Types, Type Families |
| RFC-012 | F-String Template Strings | Near Completion | Format Specifiers (`:.2f`, etc.) |
| RFC-013 | Error Code Standard Design | Near Completion | yaoxiang explain CLI Command |
| RFC-014 | Package Management System Design | Near Completion | Registry Source, Workspace Support, Dependency Overrides |
| RFC-015 | Configuration System Design | In Progress | User-Level Config, Config Merging, yaoxiang config CLI, CLI/Environment Variable Overrides |
| RFC-017 | LSP Language Server Support | Near Completion | Incremental Sync, TCP/Unix Socket, DAP Debug Adapter |
| RFC-023 | Closure Capture Model | Near Completion | Full Escape Analysis |

---

## Near-Completion RFCs (8)

Minimal remaining work; each RFC only lacks 1-2 features.

### RFC-006: Documentation Site

**Completed**: VitePress site, multi-language support (zh, en, ja, ru), CI/CD auto-deployment, sidebar/navbar configuration, search functionality

**Remaining**:
- ❌ Version switcher menu (`/v0.5/zh/` format)

### RFC-007: Unified Function Definition Syntax

**Completed**: `name: (params) -> Return = body` syntax parsing, lambda expression syntax, HM type inference, unified function definition syntax

**Remaining**:
- ❌ `yaoxiang-migrate` migration tool

### RFC-012: F-String Template Strings

**Completed**: `f"Hello {name}"` prefix syntax, variable interpolation/expression evaluation, compile-time conversion to string operations

**Remaining**:
- ❌ Full support for format specifiers (`:.2f`, etc.)

### RFC-013: Error Code Standard Design

**Completed**: Four-digit number scheme `Exxxx`, multi-language resource files (JSON i18n), `DiagnosticBuilder` generic builder, `error!` macro

**Remaining**:
- ⚠️ `yaoxiang explain` CLI command pending confirmation

### RFC-023: Closure Capture Model

**Completed**: Compiler automatic analysis of external variables referenced in closure body, Dup type direct copy, zero annotation

**Remaining**:
- ⚠️ Full escape analysis to be refined

### RFC-009: Ownership Model

**Completed**: Move semantics (default), &T/&mut T borrow tokens, clone() explicit deep copy, unsafe + *T, cross-task cycle detection, Send/Sync constraints, freeze mechanism, token conflict detection

**Remaining**:
- ❌ Branding mechanism (compiler-internal token unique identification)
- ❌ ref automatic Rc/Arc selection escape analysis

### RFC-010: Unified Type Syntax

**Completed**: Unified declaration syntax `name: type = value`, record type definition, interface definition and implementation checking, method definition syntax (Type.method), generic call `()` syntax, structural subtyping check

**Remaining**:
- ❌ Full duck typing support
- ❌ Interface composition (`Drawable & Serializable` intersection types)

### RFC-017: LSP Language Server Support

**Completed**: Standalone LSP server process, JSON-RPC communication, code completion/jump to definition/diagnostics/reference search/hover info, ghost text (Inlay Hints), semantic tokens, rename, code actions

**Remaining**:
- ❌ Incremental sync
- ❌ TCP/Unix Socket remote communication
- ❌ DAP debug adapter
- ❌ Ownership semantics visualization

---

## In-Progress RFCs (4)

Core functionality has been started, but a significant amount of work remains.

### RFC-001: Spawn Model and Error Handling

**Completed**: spawn check (spawn_placement.rs), basic concurrent executor, runtime engine/task system, Result type exists in standard library

**Remaining**:
- ❌ DAG dependency analyzer
- ❌ Full parsing and execution of `@block`, `@eager` annotations
- ❌ Resource type system (FilePath, HttpUrl, DBUrl, etc.)
- ❌ Isolated DAG independent parallelism
- ❌ Error graph visualization

### RFC-004: Multi-Position Union Binding for Curried Methods

**Completed**: Basic method binding syntax (Type.method definition), basic method call syntax sugar

**Remaining**:
- ❌ `[positions]` position index binding syntax (`[0]`, `[-1]`, etc.)
- ❌ Multi-position union binding `[0, 1]`
- ❌ Auto-currying binding
- ❌ Default binding logic

### RFC-011: Generics System Design

**Completed**: Unified signature syntax `(T: Type, R: Type) -> ...`, Type self-describing mechanism, type constraints `T: Dup + Add`, associated types (GAT), function overload specialization, monomorphization, dead code elimination

**Remaining**:
- ❌ Value-dependent types (`Vec: (n: Int) -> Type`)
- ❌ Compile-time evaluation engine (compile-time evaluation of functions in type positions)
- ❌ decreases clauses (termination checking)
- ❌ Conditional types `If: (C: Bool, T: Type, E: Type) -> Type`
- ❌ Type families

### RFC-015: Configuration System Design

**Completed**: Basic configuration module, project-level `yaoxiang.toml` parsing

**Remaining**:
- ❌ User-level config `~/.config/yaoxiang/config.toml`
- ❌ Config merging logic
- ❌ `yaoxiang config` CLI command
- ❌ CLI/environment variable overrides
- ❌ `[tool.*]` extensions
- ❌ `platform` platform constraints

---

## Not-Started RFCs (1)

### RFC-008: Runtime Concurrency Model and Scheduler Decoupling

**Completed**: Basic interpreter/VM backend, basic runtime engine (engine.rs, task.rs)

**Remaining**:
- ❌ Embedded Runtime (immediate executor)
- ❌ Standard Runtime (DAG scheduler)
- ❌ Full Runtime (work stealing)
- ❌ LLVM AOT backend
- ❌ Generic scheduler interface
- ❌ Scheduler static library linking

---

## Unimplemented Checklist (By Priority)

### High Priority (Core Language Features)

1. **DAG Dependency Analyzer** (RFC-001) — Core of spawn model
2. **Value-Dependent Types** (RFC-011) — `Vec: (n: Int) -> Type`
3. **decreases Clauses/Termination Checking** (RFC-011) — Compile-time evaluation safety guarantee
4. **Conditional Types** (RFC-011) — `If: (C: Bool, T: Type, E: Type) -> Type`
5. **`[positions]` Position Index Binding Syntax** (RFC-004)
6. **`@block`, `@eager` Annotations** (RFC-001)
7. **LLVM AOT Backend** (RFC-008)
8. **Generic Scheduler Interface** (RFC-008)

### Medium Priority (Tooling)

9. **User-Level Config** (RFC-015) — `~/.config/yaoxiang/config.toml`
10. **Config Merging Logic** (RFC-015)
11. **`yaoxiang config` CLI Command** (RFC-015)
12. **`yaoxiang explain` CLI Command** (RFC-013)
13. **`yaoxiang-migrate` Migration Tool** (RFC-007)
14. **Registry Source** (RFC-014)
15. **Integrity Verification** (RFC-014)

### Low Priority (Enhancement Features)

16. **Format Specifiers** (RFC-012) — `f"Pi: {pi:.2f}"`
17. **Version Switcher Menu** (RFC-006)
18. **Interface Composition** (RFC-010) — `Drawable & Serializable`
19. **TCP/Unix Socket Remote Communication** (RFC-017)
20. **DAP Debug Adapter** (RFC-017)
21. **Ownership Semantics Visualization** (RFC-017)
22. **Resource Type System** (RFC-001)
23. **`[tool.*]` Third-Party Tool Config Extensions** (RFC-015)
24. **`platform` Platform Constraints** (RFC-015)