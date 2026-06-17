---
title: "YaoXiang Project Roadmap"
---

# YaoXiang Project Roadmap

> **Current Version**: v0.7.2
> **Last Updated**: 2026-06-04
> **Status**: Active Development

---

## Project Overview

YaoXiang is a future-oriented programming language that adopts a bytecode compilation + VM interpretation execution architecture. Currently in the v0.7.x phase, the core compiler and runtime are essentially complete.

---

## Module Status Overview

| Module | Status | Open Issues | Test Count | Documentation |
|------|------|-----------|--------|------|
| [Lexer](./lexer.md) | ✅ Stable | 3 | 31 (150+ uncompiled) | Good |
| [Parser](./parser.md) | ✅ Stable | 5 | 285 | Good |
| [Type Checker](./typecheck.md) | ✅ Stable | 0 | 635 | Excellent |
| [Code Generation](./codegen.md) | ⚠️ Has Gaps | 5 | 13 | Medium |
| [Borrow Checker](./lifetime.md) | ✅ Stable | 3 | 83 | Good |
| [Interpreter](./interpreter.md) | ✅ Stable | 5 | ~60 | Good |
| [Runtime](./runtime.md) | ⚠️ Has Gaps | 4 | 22 | Good |
| [REPL](./repl.md) | ⚠️ Has Gaps | 3 | 0 | Good |
| [Formatter](./formatter.md) | ✅ Stable | 0 | 58 | High |
| [Language Server](./lsp.md) | ✅ Stable | 5 | 145 | Excellent |
| [Package Management](./package.md) | ✅ Stable | 4 | 137 | Good |
| [Standard Library](./std.md) | ⚠️ Has Gaps | 4 | 8 | Good |

**Total**: approximately 1,470 tests

---

## RFC Implementation Status

| RFC | Title | Status | Remaining Key Steps |
|-----|------|------|-------------|
| RFC-001 | Spawn Model and Error Handling | In Progress | DAG dependency analyzer, complete @block/@eager execution, resource type system |
| RFC-004 | Curried Methods with Multi-Position Union Binding | In Progress | [positions] syntax, multi-position union binding, automatic currying, default binding |
| RFC-006 | Documentation Site Construction and Optimization | Nearly Complete | Version switcher menu |
| RFC-007 | Unified Function Definition Syntax | Nearly Complete | yaoxiang-migrate migration tool |
| RFC-008 | Runtime Concurrency Model and Scheduler Decoupling | Not Started | LLVM AOT backend, generic scheduler interface, scheduler static library |
| RFC-009 | Ownership Model | Nearly Complete | Brand mechanism, ref escape analysis |
| RFC-010 | Unified Type Syntax | Nearly Complete | Duck typing support, interface composition (intersection types) |
| RFC-011 | Generics System Design | In Progress | Value-dependent types, compile-time evaluation engine, decreases reduction, conditional types, type families |
| RFC-012 | F-String Template Strings | Nearly Complete | Format specifiers (`:.2f` etc.) |
| RFC-013 | Error Code Specification Design | Nearly Complete | yaoxiang explain CLI command |
| RFC-014 | Package Management System Design | Nearly Complete | Registry source, workspace support, dependency overrides |
| RFC-015 | Configuration System Design | In Progress | User-level configuration, config merging, yaoxiang config CLI, command-line/environment variable overrides |
| RFC-017 | LSP Language Server Support | Nearly Complete | Incremental sync, TCP/Unix Socket, DAP debug adapter |
| RFC-023 | Closure Capture Model | Nearly Complete | Complete escape analysis |

Detailed comparison: [RFC Implementation Status](./rfc-status.md)

---

## Core Findings

### Completed Areas

1. **Compiler Frontend**: Lexical analysis, parsing, and type checking are all complete, with excellent test coverage
2. **Ownership System**: Complete borrow checker, 14/15 sub-modules fully functional
3. **Toolchain**: LSP, package management, and formatter are essentially complete
4. **Documentation Site**: Multi-language support, CI/CD automated deployment

### Major Gaps

1. **Spawn Model** (RFC-001): DAG dependency analyzer not implemented, concurrency control annotations incomplete
2. **Runtime Three-Layer Architecture** (RFC-008): Only phase A complete, LLVM AOT backend not implemented
3. **Value-Dependent Types** (RFC-011): Compile-time evaluation engine not implemented
4. **Configuration System** (RFC-015): User-level configuration not implemented
5. **Standard Library Tests**: Only 8 unit tests, seriously insufficient

---

## Next Steps

### Short Term (v0.8) — Testing and Quality

**Goal**: Fill in test coverage, fix known issues

#### Test Completion (Highest Priority)

| Module | Current Tests | Target | Specific Tasks |
|------|----------|------|----------|
| Standard Library | 8 | 100+ | Add unit tests for io/math/string/list/dict/os/time/concurrent |
| REPL | 0 | 20+ | Add tests for evaluation engine, command system, autocompletion |
| lexer | 31 (150+ uncompiled) | 180+ | Activate 11 test files in tests/ directory |
| codegen | 13 | 30+ | Add unit tests for translator.rs |
| lifetime | 83 | 100+ | Add tests for drop_semantics/clone/mut_check/ref_semantics/unsafe_check |
| interpreter | ~60 | 80+ | Add boundary condition and error path tests |
| runtime | 22 | 30+ | Add tests for facade.rs and task.rs |

#### Known Issue Fixes

| Issue | Module | Description |
|------|------|------|
| `os.chdir` does not actually change directory | std | Only checks if directory exists, does not call `std::env::set_current_dir()` |
| `string.len` returns byte count | std | `native_len` uses `s.len()` to return byte count instead of character count |
| `weak` module cannot be imported | std | Missing `StdModule` trait implementation |

#### CTE Compile-Time Evaluation Engine (Phase 1-2)

- [ ] Define `CTValue` enum and `EvalEnv` struct
- [ ] Implement basic path of `eval()`: literals, variables, binary operations, conditionals, code blocks
- [ ] Implement first version of purity analyzer
- [ ] Insert CTE call points in type checker
- [ ] Constant folding: `1 + 2 * 3` evaluates to `7` at compile-time
- [ ] Implement function inlining evaluation
- [ ] Implement `//! decreases` parsing and termination verification

---

### Medium Term (v0.9) — Core Features

**Goal**: Complete core language features, begin LLVM backend

#### CTE Compile-Time Evaluation Engine (Phase 3-4)

- [ ] Implement type-level operations for `CTValue::Type(TypeId)`
- [ ] Implement conditional type evaluation for `If: (C: Bool, T: Type, E: Type) -> Type`
- [ ] Implement `Assert(C)` → `True → Void, False → compile_error`
- [ ] Implement type-level `match`
- [ ] Parser extension: recognize `//!` and `/*! ... !*/` as reduction nodes
- [ ] VC generator: weakest precondition calculus
- [ ] Z3 SMT solver integration

#### LLVM AOT Backend (Start)

- [ ] Lock LLVM/inkwell version (LLVM 17)
- [ ] Implement stable `RtValue` / `RtContext` ABI
- [ ] Implement basic AOT compilation (serial, no concurrency)
- [ ] Integrate into CLI: `yaoxiang run --backend llvm`

#### Configuration System (RFC-015)

- [ ] Implement user-level configuration `~/.config/yaoxiang/config.toml`
- [ ] Implement configuration merge logic (project-level overrides user-level)
- [ ] Implement `yaoxiang config` CLI command (init, edit, show, reset)
- [ ] Implement command-line/environment variable overrides

#### RFC-004 Position Index Binding

- [ ] Implement `[positions]` position index binding syntax (`[0]`, `[-1]`, etc.)
- [ ] Implement multi-position union binding `[0, 1]`
- [ ] Implement automatic currying binding
- [ ] Implement default binding logic

#### Runtime Phase B (Compiler Integration)

- [ ] Implement `Result[T,E]` and `?` closure
- [ ] Implement error graph visualization (optional)

---

### Long Term (v1.0) — Production Ready

**Goal**: Complete self-hosting, production ready

#### LLVM AOT Backend (Complete)

- [ ] Implement DAG metadata + single-threaded scheduling
- [ ] Implement multi-threaded parallel scheduling + granularity control
- [ ] Implement lazy task creation
- [ ] Implement resource type (Resource) side-effect abstraction
- [ ] Implement error propagation / error graph

#### Self-Hosting

- [ ] Lexer → Parser → TypeChecker → Codegen gradually replaced
- [ ] Cross-validation: results from two compilers are consistent
- [ ] Complete self-hosting

#### Production Ready

- [ ] API freeze
- [ ] Complete documentation and tutorials
- [ ] Performance optimization
- [ ] Edge case fixes

---

## Existing Planning Documents

- [Compile-Time Evaluation Engine (CTE)](./ongoing/compile-time-evaluation-engine.md)
- [LLVM AOT Compiler](./ongoing/RFC-018-llvm-aot-compiler-implementation.md)