---
title: "YaoXiang Project Roadmap"
---

# YaoXiang Project Roadmap

> **Current Version**: v0.7.2
> **Last Updated**: 2026-06-04
> **Status**: In Active Development

---

## Project Overview

YaoXiang is a future-oriented programming language, adopting a bytecode compilation + VM interpretation architecture. Currently in v0.7.x stage, the core compiler and runtime are basically complete.

---

## Module Status Overview

| Module | Status | Remaining Tasks | Test Count | Documentation |
|--------|--------|-----------------|------------|---------------|
| [Lexer](./lexer.md) | ✅ Stable | 3 | 31 (150+ uncompiled) | Good |
| [Parser](./parser.md) | ✅ Stable | 5 | 285 | Good |
| [Type Checker](./typecheck.md) | ✅ Stable | 0 | 635 | Excellent |
| [Code Generation](./codegen.md) | ⚠️ Has Gaps | 5 | 13 | Medium |
| [Borrow Checker](./lifetime.md) | ✅ Stable | 4 | 83 | Good |
| [Interpreter](./interpreter.md) | ✅ Stable | 5 | ~60 | Good |
| [Runtime](./runtime.md) | ⚠️ Has Gaps | 4 | 22 | Good |
| [REPL](./repl.md) | ⚠️ Has Gaps | 3 | 0 | Good |
| [Formatter](./formatter.md) | ✅ Stable | 0 | 58 | High |
| [Language Server](./lsp.md) | ✅ Stable | 5 | 145 | Excellent |
| [Package Manager](./package.md) | ✅ Stable | 4 | 137 | Good |
| [Standard Library](./std.md) | ⚠️ Has Gaps | 4 | 8 | Good |

**Total**: ~1,470 tests

---

## RFC Implementation Status

| RFC | Title | Status | Remaining Key Steps |
|-----|-------|--------|---------------------|
| RFC-001 | spawn Model and Error Handling | In Progress | DAG Dependency Analyzer, @block/@eager Complete Execution, Resource Type System |
| RFC-004 | Multi-Position Union Binding for Curried Methods | In Progress | [positions] Syntax, Multi-Position Union Binding, Auto Currying, Default Binding |
| RFC-006 | Documentation Site Construction and Optimization | Near Complete | Version Switch Menu |
| RFC-007 | Function Definition Syntax Unification | Near Complete | yaoxiang-migrate Migration Tool |
| RFC-008 | Runtime Concurrency Model and Scheduler Decoupling | Not Started | LLVM AOT Backend, Generic Scheduler Interface, Scheduler Static Library |
| RFC-009 | Ownership Model | Near Complete | freeze Mechanism, Branding Mechanism, ref Escape Analysis |
| RFC-010 | Unified Type Syntax | Near Complete | Duck Typing Support, Interface Combination (Intersection Types) |
| RFC-011 | Generics System Design | In Progress | Value-Dependent Types, Compile-Time Evaluation Engine, decreases Specification, Conditional Types, Type Families |
| RFC-012 | F-String Template Strings | Near Complete | Format Specifiers (`:.2f`, etc.) |
| RFC-013 | Error Code Specification Design | Near Complete | yaoxiang explain CLI Command |
| RFC-014 | Package Management System Design | Near Complete | Registry Source, Workspace Support, Dependency Override |
| RFC-015 | Configuration System Design | In Progress | User-Level Config, Config Merge, yaoxiang config CLI, Command Line/Environment Variable Override |
| RFC-017 | LSP Language Server Support | Near Complete | Incremental Sync, TCP/Unix Socket, DAP Debug Adapter |
| RFC-023 | Closure Capture Model | Near Complete | Complete Escape Analysis |

Detailed Comparison: [RFC Implementation Status](./rfc-status.md)

---

## Key Findings

### Completed Areas

1. **Compiler Frontend**: Lexical analysis, parsing, and type checking all complete, excellent test coverage
2. **Ownership System**: Complete borrow checker, 14/15 submodules functional
3. **Toolchain**: LSP, package management, and formatter basically complete
4. **Documentation Site**: Multi-language support, CI/CD auto deployment

### Main Gaps

1. **spawn Model** (RFC-001): DAG dependency analyzer not implemented, concurrency control annotations incomplete
2. **Runtime Three-Layer Architecture** (RFC-008): Only Phase A complete, LLVM AOT backend not implemented
3. **Value-Dependent Types** (RFC-011): Compile-time evaluation engine not implemented
4. **Configuration System** (RFC-015): User-level configuration not implemented
5. **Standard Library Tests**: Only 8 unit tests, severely insufficient

---

## Next Steps

### Short Term (v0.8) — Testing and Quality

**Goal**: Fill test coverage, fix known issues

#### Test Coverage (Highest Priority)

| Module | Current Tests | Target | Specific Tasks |
|--------|---------------|--------|----------------|
| Standard Library | 8 | 100+ | Add unit tests for io/math/string/list/dict/os/time/concurrent |
| REPL | 0 | 20+ | Add evaluation engine, command system, and auto-completion tests |
| Lexer | 31 (150+ uncompiled) | 180+ | Activate 11 test files in tests/ directory |
| Codegen | 13 | 30+ | Add translator.rs unit tests |
| Lifetime | 83 | 100+ | Add drop_semantics/clone/mut_check/ref_semantics/unsafe_check tests |
| Interpreter | ~60 | 80+ | Add boundary conditions and error path tests |
| Runtime | 22 | 30+ | Add facade.rs and task.rs tests |

#### Known Issues Fixes

| Issue | Module | Description |
|-------|--------|-------------|
| `os.chdir` doesn't actually change directory | std | Only checks if directory exists, doesn't call `std::env::set_current_dir()` |
| `string.len` returns byte count | std | `native_len` uses `s.len()` which returns bytes instead of character count |
| `weak` module cannot be imported | std | Missing `StdModule` trait implementation |

#### CTE Compile-Time Evaluation Engine (Phase 1-2)

- [ ] Define `CTValue` enum and `EvalEnv` struct
- [ ] Implement basic paths for `eval()`: literals, variables, binary operations, conditionals, code blocks
- [ ] Implement first version of purity analyzer
- [ ] Insert CTE call points in type checker
- [ ] Constant folding: `1 + 2 * 3` computed at compile time as `7`
- [ ] Implement function inline evaluation
- [ ] Implement `//! decreases` parsing and termination verification

---

### Medium Term (v0.9) — Core Features

**Goal**: Complete core language features, start LLVM backend

#### CTE Compile-Time Evaluation Engine (Phase 3-4)

- [ ] Implement type-level operations for `CTValue::Type(TypeId)`
- [ ] Implement conditional type evaluation for `If: (C: Bool, T: Type, E: Type) -> Type`
- [ ] Implement `Assert(C)` → `True → Void, False → compile_error`
- [ ] Implement type-level `match`
- [ ] Parser extension: recognize `//!` and `/*! ... !*/` as specification nodes
- [ ] VC Generator: weakest precondition calculus
- [ ] Z3 SMT Solver integration

#### LLVM AOT Backend (Start)

- [ ] Pin LLVM/inkwell version (LLVM 17)
- [ ] Implement stable ABI for `RtValue` / `RtContext`
- [ ] Implement basic AOT compilation (serial, no concurrency)
- [ ] Integrate into CLI: `yaoxiang run --backend llvm`

#### Configuration System (RFC-015)

- [ ] Implement user-level config `~/.config/yaoxiang/config.toml`
- [ ] Implement config merge logic (project-level overrides user-level)
- [ ] Implement `yaoxiang config` CLI command (init, edit, show, reset)
- [ ] Implement command line/environment variable override

#### RFC-004 Position Index Binding

- [ ] Implement `[positions]` position index binding syntax (`[0]`, `[-1]`, etc.)
- [ ] Implement multi-position union binding `[0, 1]`
- [ ] Implement auto currying binding
- [ ] Implement default binding logic

#### Runtime Phase B (Compiler Integration)

- [ ] Implement compile-time checks for `@block` and `@eager` annotations
- [ ] Implement `Result[T,E]` and `?` closed loop
- [ ] Implement error graph visualization (optional)

---

### Long Term (v1.0) — Production Ready

**Goal**: Complete bootstrap, production ready

#### LLVM AOT Backend (Complete)

- [ ] Implement DAG metadata + single-threaded scheduling
- [ ] Implement multi-threaded parallel scheduling + granularity control
- [ ] Implement lazy task creation scheduling
- [ ] Implement Resource type副作用 abstraction
- [ ] Implement error propagation/error graphs

#### Bootstrap

- [ ] Lexer → Parser → TypeChecker → Codegen stepwise replacement
- [ ] Cross-validation: two compiler results match
- [ ] Full bootstrap

#### Production Ready

- [ ] API Freeze
- [ ] Complete documentation and tutorials
- [ ] Performance optimization
- [ ] Edge case fixes

---

## Existing Planning Documents

- [Compile-Time Evaluation Engine (CTE)](./ongoing/compile-time-evaluation-engine.md)
- [LLVM AOT Compiler](./ongoing/RFC-018-llvm-aot-compiler-implementation.md)