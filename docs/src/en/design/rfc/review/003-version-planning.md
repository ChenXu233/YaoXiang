---
title: "RFC-003: Version Planning"
---

# RFC-003: Version Planning

> **Status**: Rejected
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2025-01-06
> **Rejection Date**: 2026-06-01

## ⚠️ Rejection Reason

**This RFC does not conform to RFC standards and has been rejected.**

### Problem Analysis

| Problem | Description |
|---------|-------------|
| **Wrong document type** | RFC is a decision document about "why this design", this article is a project management document about "where we are and where we're going" |
| **Lacks design decisions** | No trade-off analysis of technical choices, no comparison of alternatives, no design rationale |
| **Over-planning** | Planned 9 versions (v0.1-v1.0), but only 1.5 completed; v0.3 onwards are speculations |
| **High maintenance cost** | Success indicators table has 50 states, difficult to maintain |
| **Version number confusion** | Skipped v0.8, no explanation |

### Correct Document Type

This document should be managed as a **Roadmap**, not an RFC. Roadmaps document facts and short-term plans; RFCs document design decisions.

---

> **The following is the original content, preserved for reference.**

## Summary

YaoXiang's version release plan, a roadmap from v0.1 to v1.0.

**Core Goals**:
1. **Bytecode Compilation**: Support REPL and incremental compilation
2. **Bootstrap**: Use YaoXiang to write the YaoXiang compiler
3. **AOT Compilation**: Compile bytecode to native machine code

## 1. Motivation

### Why is version planning needed?

1. **Project Management**: Decompose goals into executable milestones
2. **User Expectations**: Let users understand the language's development stages
3. **Resource Allocation**: Clarify focus areas for each phase
4. **Risk Control**: Identify problems early and adjust direction

### Core Design Decisions

- **Bytecode First**: Implement interpreted execution first, then consider AOT
- **Incremental Delivery**: Each version has usable functionality
- **Backward Compatibility**: API may change before v1.0, but with advance notice
- **Bootstrap Verification**: Prove language expressiveness through bootstrapping
- **Performance Tiers**: Get it running first, then optimize

## 2. Component Status (Phase)

| Phase | Module | Status | Location | Last Updated |
|-------|--------|--------|----------|--------------|
| P1 | Lexer | ✅ Complete | `src/frontend/lexer/` | 2025-01-23 |
| P2 | Type Checker | ✅ Complete | `src/frontend/typecheck/` | 2025-01-23 |
| P3 | Bytecode Generator | ✅ Complete | `src/middle/codegen/` | 2025-01-25 |
| P4 | Virtual Machine | ✅ Complete | `src/middle/` | 2025-01-25 |
| P4.1 | Task System | ✅ Complete | `src/backends/runtime/task.rs` | 2025-01-23 |
| P4.2 | DAG Scheduler | 🔶 Design Complete | `.claude/plan/flow-scheduler-implementation.md` | 2026-01-04 |
| P5 | Standard Library | ⚠️ Partial | `src/std/` | 2025-01-23 |
| P6 | TUI REPL | ✅ Complete | `src/backends/dev/repl/` | 2025-01-24 |
| P7 | Generics System | ✅ Complete | `docs/design/rfc/011-generic-type-system.md` | 2025-01-25 |

**Core Achievements**:
- ✅ Complete compiler frontend implementation (P1-P2)
- ✅ Bytecode generation and VM complete (P3-P4)
- ✅ Basic task system complete (P4.1)
- ✅ TUI REPL development complete (P6)
- ✅ Generics system design complete (P7)

**Next Priority**: Implement FlowScheduler scheduler → Complete standard library (P5) → v0.1 release

## 3. Version Roadmap

### v0.1: Runnable Milestone ✅

**Status**: Basic completion (2025-01-25)

**Completed**:
- ✅ Complete lexical analysis, parsing, type checking
- ✅ Bytecode generation functional
- ✅ VM can interpret and execute basic programs
- ✅ Basic print function
- ✅ TUI REPL complete
- ✅ Basic task system (Task/Scheduler)

```
$ yaoxiang run hello.yx
Hello, YaoXiang!
```

**Technical Highlights**:
- Three-layer runtime architecture design complete
- Task system fully implemented
- Modern TUI REPL interface
- Unified type syntax + Generics system design

**Not Included**: Complete DAG scheduling (basic scheduler already implemented)

### v0.2: FlowScheduler Scheduler 🚧

**Goal**: Implement complete dependency-aware scheduler

- ✅ Design documentation complete
- 🔶 Implementation in progress
- [ ] DAG node and graph implementation
- [ ] Work-stealing algorithm
- [ ] libuv IO scheduling engine
- [ ] Lazy evaluation strategy
- [ ] spawn syntax support

**Technical Focus**:
- FlowScheduler architecture implementation
- Industrial-grade IO scheduling (libuv)
- Zero-cost abstraction

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
- Zero-cost abstraction
- Function overloading + inline optimization

### v0.5: Standard Library Enhancement 📋

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

**Goal**: Rewrite core modules in YaoXiang

- Lexer → Parser → TypeChecker → Codegen gradual replacement
- Cross-validation: two compilers produce consistent results

### v1.0: Production Ready 📋

**Goal**: Stable release

- Complete bootstrap
- AOT compilation (LLVM backend)
- Production ready

## 4. Three-Layer Compilation Strategy

| Layer | Version | Input | Output | Description |
|-------|---------|-------|--------|-------------|
| L1: Bytecode | v0.1+ | Source code (.yx) | Bytecode (.yxb) | VM interpreted execution |
| L2: Bootstrap | v0.9+ | YaoXiang source | Bytecode | Self-compilation |
| L3: AOT | v1.0+ | Source/Bytecode | Machine code | Native performance |

**Rationale for Bytecode First**:
1. **REPL Support**: Compile input code immediately, interactive development
2. **Incremental Compilation**: Modifying a single function only requires recompiling that part
3. **Platform Independent**: .yxb files run cross-platform, only need VM for target platform

## 5. Dependency Strategy

**Short Term**: Call Rust libraries, reuse crates.io (Cargo parasitism)

**Current Dependencies**:
- Concurrency: parking_lot, crossbeam, rayon
- Data Structures: indexmap, hashbrown, smallvec
- Networking: tokio
- Serialization: serde, ron

**Long Term**: Build own standard library and package manager

## 6. Toolchain

| Version | Tool | Status |
|---------|------|--------|
| v0.1 | yaoxiang-cli | ✅ Complete |
| v0.1 | TUI REPL | ✅ Complete |
| v0.2 | yaoxiang-debug | 🚧 In design |
| v0.3 | yaoxiang-fmt | 📋 Planned |
| v0.3 | yaoxiang-lsp (basic) | 📋 Planned |
| v0.5 | yaoxiang-clippy | 📋 Planned |
| v1.0 | Complete toolchain | 📋 Planned |

## 7. Success Indicators

| Indicator | v0.1 | v0.2 | v0.3 | v0.5 | v1.0 |
|-----------|------|------|------|------|------|
| End-to-end running | ✅ | ✅ | ✅ | ✅ | ✅ |
| Basic task system | ✅ | ✅ | ✅ | ✅ | ✅ |
| FlowScheduler | ❌ | 🚧 | ✅ | ✅ | ✅ |
| Concurrency support | ⚠️ | 🚧 | ✅ Basic | ✅ Complete | ✅ |
| Standard library | Basic | Basic | Basic | Enhanced | Complete |
| Generics system | ⚠️ | ⚠️ | 🚧 | ✅ | ✅ |
| TUI REPL | ✅ | ✅ | ✅ | ✅ | ✅ |
| Bootstrap | ❌ | ❌ | ❌ | ❌ | ✅ |
| AOT | ❌ | ❌ | ❌ | ❌ | ✅ |
| Code coverage | 60% | 70% | 80% | 90% | 95% |

**Legend**:
- ✅ Completed
- 🚧 In Progress
- ⚠️ Partially Complete
- 📋 Planned

## 8. Open Questions

- [ ] JIT vs AOT timing selection
- [ ] Package manager design
- [ ] Bootstrap module replacement order
- [ ] AOT backend selection (LLVM vs custom)

## 9. Version Release Criteria

**v0.x Series**:
- Functionality complete but may have edge case issues
- API may change
- For learning and experimentation only

**v1.0**:
- All core functionality stable
- API frozen
- Suitable for production use
- Complete documentation and tutorials

## References

- [Semantic Versioning 2.0.0](https://semver.org/lang/zh-CN/)
- [Rust Release Model](https://forge.rust-lang.org/release.html)
- [RFC-001: Concurrency Model and Error Handling](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model](./008-runtime-concurrency-model.md)