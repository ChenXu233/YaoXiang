---
title: 'RFC-003: Version Planning'
status: 'Rejected'
author: 'Chen Xu'
created: '2025-01-05'
updated: '2025-01-06'
---

# RFC-003: Version Planning

> **Rejection Date**: 2026-06-01

## ⚠️ Rejection Reason

**This RFC does not comply with RFC standards and has been rejected.**

### Problem Analysis

| Issue             | Description                                                                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **Wrong Doc Type** | RFCs are decision documents about "why this design", this document is a project management document about "where we are, where we're going" |
| **Lack of Design Decisions** | No trade-off analysis for technical choices, no alternative comparison, no design rationale                     |
| **Over-planning**      | Planned 9 versions (v0.1-v1.0), but only 1.5 are complete; v0.3 and beyond are all speculation                  |
| **High Maintenance Cost** | Success metrics table has 50 states, difficult to maintain                                                         |
| **Version Number Chaos** | Skipped v0.8, no explanation provided                                                                 |

### Correct Document Type

This document should be managed as a **Project Roadmap**, not an RFC. Roadmaps record facts and short-term plans, RFCs record design decisions.

---

> **The following is the original content, preserved for reference.**

## Abstract

YaoXiang's version release plan, a roadmap from v0.1 to v1.0.

**Core Objectives**:

1. **Bytecode Compilation**: Support REPL and incremental compilation
2. **Bootstrap**: Write the YaoXiang compiler in YaoXiang
3. **AOT Compilation**: Compile bytecode to native machine code

## 1. Motivation

### Why Do We Need Version Planning?

1. **Project Management**: Decompose goals into executable milestones
2. **User Expectations**: Let users understand the language's development stages
3. **Resource Allocation**: Clarify focus areas for each phase
4. **Risk Control**: Detect problems early and adjust direction

### Core Design Decisions

- **Bytecode First**: Implement interpreter execution first, then consider AOT
- **Incremental Delivery**: Each version has usable features
- **Backward Compatibility**: APIs may change before v1.0, but will be announced in advance
- **Bootstrap Verification**: Prove the language's expressiveness through self-hosting
- **Performance Tiers**: Get it working first, then optimize

## 2. Component Status (Phase)

| Phase | Module            | Status        | Location                                           | Last Updated  |
| ----- | ----------------- | ------------- | -------------------------------------------------- | ------------- |
| P1    | Lexer             | ✅ Complete   | `src/frontend/lexer/`                              | 2025-01-23   |
| P2    | Type Checker      | ✅ Complete   | `src/frontend/typecheck/`                          | 2025-01-23   |
| P3    | Bytecode Generator| ✅ Complete   | `src/middle/codegen/`                              | 2025-01-25   |
| P4    | Virtual Machine   | ✅ Complete   | `src/middle/`                                      | 2025-01-25   |
| P4.1  | Task System       | ✅ Complete   | `src/backends/runtime/task.rs`                     | 2025-01-23   |
| P4.2  | DAG Scheduler     | 🔶 Design Done| `.claude/plan/flow-scheduler-implementation.md`    | 2026-01-04   |
| P5    | Standard Library  | ⚠️ Partial    | `src/std/`                                         | 2025-01-23   |
| P6    | TUI REPL          | ✅ Complete   | `src/backends/dev/repl/`                           | 2025-01-24   |
| P7    | Generics System   | ✅ Complete   | `docs/design/rfc/011-generic-type-system.md`       | 2025-01-25   |

**Core Achievements**:

- ✅ Compiler frontend fully implemented (P1-P2)
- ✅ Bytecode generation and virtual machine completed (P3-P4)
- ✅ Basic task system completed (P4.1)
- ✅ TUI REPL development completed (P6)
- ✅ Generics system design completed (P7)

**Next Priority**: Implement FlowScheduler → Improve standard library (P5) → v0.1 release

## 3. Version Roadmap

### v0.1: Runnable Milestone ✅

**Status**: Basically complete (2025-01-25)

**Completed**:

- ✅ Complete lexer, parser, type checking
- ✅ Bytecode generation available
- ✅ Virtual machine can interpret and execute basic programs
- ✅ Basic print function
- ✅ TUI REPL completed
- ✅ Basic task system (Task/Scheduler)

```
$ yaoxiang run hello.yx
Hello, YaoXiang!
```

**Technical Highlights**:

- Three-layer runtime architecture design completed
- Task system fully implemented
- TUI REPL modern interface
- Unified type syntax + generics system design

**Not Included**: Full DAG scheduling (basic scheduler already implemented)

### v0.2: FlowScheduler 🚧

**Goal**: Implement complete dependency-aware scheduler

- ✅ Design document completed
- 🔶 Implementation in progress
- [ ] DAG nodes and graph implementation
- [ ] Work-stealing algorithm
- [ ] libuv IO scheduling engine
- [ ] Lazy evaluation strategy
- [ ] spawn syntax support

**Technical Focus**:

- FlowScheduler architecture implementation
- Industrial-grade IO scheduling (libuv)
- Zero-cost abstractions

### v0.3: Concurrency Preview 📋

**Goal**: Support basic concurrency

- DAG task dependency graph
- Basic scheduler
- spawn concurrency

### v0.4: Generics System 📋

**Goal**: Complete generics capability

- [ ] RFC-011 Phase 1: Basic generics
- [ ] RFC-011 Phase 2: Type constraints
- [ ] RFC-011 Phase 3: Associated types
- [ ] RFC-011 Phase 4: Const generics
- [ ] RFC-011 Phase 5: Conditional types

**Technical Focus**:

- Dead code elimination
- Zero-cost abstractions
- Function overloading + inlining optimization

### v0.5: Standard Library Improvement 📋

**Goal**: Usability improvement

- IO, dictionary, networking modules
- Toolchain (fmt, basic LSP)
- Performance optimization

### v0.6: Error Handling System 📋

**Goal**: Complete error handling

- [ ] RFC-001 implementation
- [ ] Result type system
- [ ] Error graph visualization
- [ ] DAG error propagation

### v0.7: Stable Version 📋

**Goal**: API趋于稳定

- Complete documentation
- Toolchain improvement
- Edge case fixes

### v0.9: Bootstrap Start 📋

**Goal**: Core modules rewritten in YaoXiang

- Lexer → Parser → TypeChecker → Codegen progressively replaced
- Cross-validation: results from both compilers match

### v1.0: Production Ready 📋

**Goal**: Stable release

- Complete bootstrap
- AOT compilation (LLVM backend)
- Production ready

## 4. Three-Layer Compilation Strategy

| Layer         | Version | Input         | Output          | Description              |
| ------------- | ------- | ------------- | --------------- | ------------------------ |
| L1: Bytecode  | v0.1+   | Source (.yx)  | Bytecode (.yxb) | VM interpreted execution |
| L2: Bootstrap | v0.9+   | YaoXiang src  | Bytecode        | Self-compilation         |
| L3: AOT       | v1.0+   | Source/Bytecode| Native code    | Native performance       |

**Reasons for Bytecode First**:

1. **REPL Support**: Compile input code immediately, interactive development
2. **Incremental Compilation**: Modifying a single function only requires recompiling that part
3. **Platform Independence**: .yxb files run cross-platform, only need the corresponding platform VM

## 5. Dependency Strategy

**Short-term**: Call Rust libraries, reuse crates.io (Cargo parasitic)

**Current Dependencies**:

- Concurrency: parking_lot, crossbeam, rayon
- Data structures: indexmap, hashbrown, smallvec
- Networking: tokio
- Serialization: serde, ron

**Long-term**: Build our own standard library and package manager

## 6. Toolchain

| Version | Tool                   | Status      |
| ------- | ---------------------- | ----------- |
| v0.1    | yaoxiang-cli           | ✅ Complete |
| v0.1    | TUI REPL               | ✅ Complete |
| v0.2    | yaoxiang-debug         | 🚧 Designing |
| v0.3    | yaoxiang-fmt           | 📋 Planned  |
| v0.3    | yaoxiang-lsp (basic)   | 📋 Planned  |
| v0.5    | yaoxiang-clippy        | 📋 Planned  |
| v1.0    | Complete toolchain     | 📋 Planned  |

## 7. Success Metrics

| Metric            | v0.1 | v0.2 | v0.3    | v0.5    | v1.0 |
| ----------------- | ---- | ---- | ------- | ------- | ---- |
| End-to-end run    | ✅   | ✅   | ✅      | ✅      | ✅   |
| Basic task system | ✅   | ✅   | ✅      | ✅      | ✅   |
| FlowScheduler     | ❌   | 🚧   | ✅      | ✅      | ✅   |
| Concurrency       | ⚠️   | 🚧   | ✅ Basic| ✅ Full | ✅   |
| Standard library  | Basic| Basic| Basic   | Improved| Full |
| Generics system   | ⚠️   | ⚠️   | 🚧      | ✅      | ✅   |
| TUI REPL          | ✅   | ✅   | ✅      | ✅      | ✅   |
| Bootstrap         | ❌   | ❌   | ❌      | ❌      | ✅   |
| AOT               | ❌   | ❌   | ❌      | ❌      | ✅   |
| Code coverage     | 60%  | 70%  | 80%     | 90%     | 95%  |

**Legend**:

- ✅ Completed
- 🚧 In Progress
- ⚠️ Partial
- 📋 Planned

## 8. Open Questions

- [ ] JIT vs AOT timing choice
- [ ] Package manager design
- [ ] Bootstrap module replacement order
- [ ] AOT backend selection (LLVM vs custom)

## 9. Version Release Criteria

**v0.x Series**:

- Feature complete but may have edge case issues
- API may change
- For learning and experimentation only

**v1.0**:

- All core features stable
- API frozen
- Suitable for production use
- Complete documentation and tutorials

## References

- [Semantic Versioning 2.0.0](https://semver.org/lang/zh-CN/)
- [Rust Release Process](https://forge.rust-lang.org/release.html)
- [RFC-001: Concurrency Model and Error Handling](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model](./008-runtime-concurrency-model.md)
