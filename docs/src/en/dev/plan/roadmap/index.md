---
title: "YaoXiang Project Roadmap"
---

# YaoXiang Project Roadmap

> **Current Version**: v0.7.2
> **Last Updated**: 2026-06-04
> **Status**: Active Development

---

## Project Overview

YaoXiang is a future-oriented programming language, using bytecode compilation + VM execution architecture. Currently at v0.7.x stage, the core compiler and runtime are essentially complete.

---

## Module Status Overview

| Module | Status | Remaining Items | Tests | Documentation |
|--------|---------|-----------------|-------|---------------|
| [Lexer](./lexer.md) | ✅ Stable | 3 | 31 (150+ uncompiled) | Good |
| [Parser](./parser.md) | ✅ Stable | 5 | 285 | Good |
| [Type Checker](./typecheck.md) | ✅ Stable | 0 | 635 | Excellent |
| [Code Generation](./codegen.md) | ⚠️ Gaps | 5 | 13 | Medium |
| [Borrow Checker](./lifetime.md) | ✅ Stable | 3 | 83 | Good |
| [Interpreter](./interpreter.md) | ✅ Stable | 5 | ~60 | Good |
| [Runtime](./runtime.md) | ⚠️ Gaps | 4 | 22 | Good |
| [REPL](./repl.md) | ⚠️ Gaps | 3 | 0 | Good |
| [Formatter](./formatter.md) | ✅ Stable | 0 | 58 | High |
| [Language Server](./lsp.md) | ✅ Stable | 5 | 145 | Excellent |
| [Package Management](./package.md) | ✅ Stable | 4 | 137 | Good |
| [Standard Library](./std.md) | ⚠️ Gaps | 4 | 8 | Good |

**Total**: ~1,470 tests

---

## RFC Implementation Status

| RFC | Title | Status | Remaining Key Steps |
|-----|-------|--------|---------------------|
| RFC-001 | Spawn Model and Error Handling | In Progress | DAG Dependency Analyzer, @block/@eager Full Execution, Resource Type System |
| RFC-004 | Multi-Position Union Binding for Curried Methods | In Progress | [positions] Syntax, Multi-Position Union Binding, Auto-Currying, Default Binding |
| RFC-006 | Documentation Site Construction and Optimization | Near Completion | Version Switch Menu |
| RFC-007 | Function Definition Syntax Unification | Near Completion | yaoxiang-migrate Migration Tool |
| RFC-008 | Runtime Concurrency Model and Scheduler Decoupling | Not Started | LLVM AOT Backend, Generic Scheduler Interface, Scheduler Static Library |
| RFC-009 | Ownership Model | Near Completion | Branding Mechanism, ref Escape Analysis |
| RFC-010 | Unified Type Syntax | Near Completion | Duck Typing Support, Interface Combination (Intersection Types) |
| RFC-011 | Generics System Design | In Progress | Value-Dependent Types, Compile-Time Evaluation Engine, decreases Specification, Conditional Types, Type Families |
| RFC-012 | F-String Template Strings | Near Completion | Format Specifiers (`:.2f`, etc.) |
| RFC-013 | Error Code Specification Design | Near Completion | yaoxiang explain CLI Command |
| RFC-014 | Package Management System Design | Near Completion | Registry Source, Workspace Support, Dependency Override |
| RFC-015 | Configuration System Design | In Progress | User-Level Configuration, Configuration Merge, yaoxiang config CLI, CLI/Environment Variable Override |
| RFC-017 | LSP Language Server Support | Near Completion | Incremental Sync, TCP/Unix Socket, DAP Debug Adapter |
| RFC-023 | Closure Capture Model | Near Completion | Complete Escape Analysis |

Detailed comparison: [RFC Implementation Status](./rfc-status.md)

---

## Key Findings

### Completed Areas

1. **Compiler Frontend**: Lexical analysis, parsing, type checking all complete, excellent test coverage
2. **Ownership System**: Complete borrow checker, 14/15 submodules feature-complete
3. **Toolchain**: LSP, package management, formatter basically complete
4. **Documentation Site**: Multi-language support, CI/CD auto-deployment

### Main Gaps

1. **Spawn Model** (RFC-001): DAG dependency analyzer not implemented, concurrency control annotations incomplete
2. **Runtime Three-Layer Architecture** (RFC-008): Only Phase A complete, LLVM AOT backend not implemented
3. **Value-Dependent Types** (RFC-011): Compile-time evaluation engine not implemented
4. **Configuration System** (RFC-015): User-level configuration not implemented
5. **Standard Library Tests**: Only 8 unit tests, severely insufficient

---

## Next Steps

### Short-Term (v0.8) — Testing and Quality

**Goal**: Fill in test coverage, fix known issues

#### Test Filling (Highest Priority)

| Module | Current Tests | Target | Specific Tasks |
|--------|---------------|--------|----------------|
| Standard Library | 8 | 100+ | Add unit tests for io/math/string/list/dict/os/time/concurrent |
| REPL | 0 | 20+ | Supplement evaluation engine, command system, auto-completion tests |
| Lexer | 31 (150+ uncompiled) | 180+ | Activate 11 test files in tests/ directory |
| Codegen | 13 | 30+ | Supplement translator.rs unit tests |
| Lifetime | 83 | 100+ | Supplement drop_semantics/clone/mut_check/ref_semantics/unsafe_check tests |
| Interpreter | ~60 | 80+ | Supplement boundary conditions and error path tests |
| Runtime | 22 | 30+ | Supplement facade.rs and task.rs tests |

#### Known Issue Fixes

| Issue | Module | Description |
|-------|--------|-------------|
| `os.chdir` doesn't actually change directory | std | Only checks if directory exists, doesn't call `std::env::set_current_dir()` |
| `string.len` returns byte count | std | `native_len` uses `s.len()` returns bytes instead of character count |
| `weak` module cannot be imported | std | Missing `StdModule` trait implementation |

#### CTE Compile-Time Evaluation Engine (Phase 1-2)

- [ ] Define `CTValue` enum and `EvalEnv` struct
- [ ] Implement basic paths for `eval()`: literals, variables, binary operations, conditionals, code blocks
- [ ] Implement first version of purity analyzer
- [ ] Insert CTE call points in type checker
- [ ] Constant folding: `1 + 2 * 3` computed at compile-time as `7`
- [ ] Implement function inline evaluation
- [ ] Implement `//! decreases` parsing and termination verification

---

### Mid-Term (v0.9) — Core Features

**Goal**: Complete core language features, start LLVM backend

#### CTE Compile-Time Evaluation Engine (Phase 3-4)

- [ ] Implement type-level operations for `CTValue::Type(TypeId)`
- [ ] Implement conditional type evaluation for `If: (C: Bool, T: Type, E: Type) -> Type`
- [ ] Implement `Assert(C)` → `True → Void, False → compile_error`
- [ ] Implement type-level `match`
- [ ] Parser extension: recognize `//!` and `/*! ... !*/` as specification nodes
- [ ] VC Generator: Weakest Precondition Calculus
- [ ] Z3 SMT Solver Integration

#### LLVM AOT Backend (Start)

- [ ] Lock LLVM/inkwell versions (LLVM 17)
- [ ] Implement stable ABI for `RtValue` / `RtContext`
- [ ] Implement basic AOT compilation (serial, no concurrency)
- [ ] Integrate into CLI: `yaoxiang run --backend llvm`

#### Configuration System (RFC-015)

- [ ] Implement user-level configuration `~/.config/yaoxiang/config.toml`
- [ ] Implement configuration merge logic (project-level overrides user-level)
- [ ] Implement `yaoxiang config` CLI command (init, edit, show, reset)
- [ ] Implement CLI/environment variable override

#### RFC-004 Position Index Binding

- [ ] Implement `[positions]` position index binding syntax (`[0]`, `[-1]`, etc.)
- [ ] Implement multi-position union binding `[0, 1]`
- [ ] Implement auto-curry binding
- [ ] Implement default binding logic

#### Runtime Phase B (Compiler Integration)

- [ ] Implement compile-time checks for `@block`, `@eager` annotations
- [ ] Implement `Result[T,E]` and `?` closed loop
- [ ] Implement error graph visualization (optional)

---

### Long-Term (v1.0) — Production Ready

**Goal**: Complete bootstrapping, production ready

#### LLVM AOT Backend (Complete)

- [ ] Implement DAG metadata + single-threaded scheduling
- [ ] Implement multi-threaded parallel scheduling + granularity control
- [ ] Implement lazy scheduling (Lazy Task Creation)
- [ ] Implement Resource type副作用 abstraction
- [ ] Implement error propagation/error graph

#### Bootstrapping

- [ ] Replace step by step: Lexer → Parser → TypeChecker → Codegen
- [ ] Cross-validation: two compiler results match
- [ ] Complete bootstrapping

#### Production Ready

- [ ] API Freeze
- [ ] Complete documentation and tutorials
- [ ] Performance optimization
- [ ] Edge case fixes

---

## Existing Plan Documents

- [Compile-Time Evaluation Engine (CTE)](./ongoing/compile-time-evaluation-engine.md)
- [LLVM AOT Compiler](./ongoing/RFC-018-llvm-aot-compiler-implementation.md)