---
title: 'RFC-003: Version Planning'
---

# RFC-003: Version Planning

> **Status**: Long-term Review
> **Author**: ChenXu
> **Created Date**: 2025-01-05
> **Last Updated**: 2025-01-06

## Summary

Version release plan for YaoXiang language, roadmap from v0.1 to v1.0.

**Core Goals**:
1. **Bytecode Compilation**: Support REPL and progressive compilation
2. **Bootstrap**: Write YaoXiang compiler in YaoXiang
3. **AOT Compilation**: Compile bytecode to native machine code

## 1. Motivation

### Why version planning is needed?

1. **Project Management**: Decompose goals into executable milestones
2. **User Expectations**: Let users understand language development stages
3. **Resource Allocation**: Clarify focus of each phase
4. **Risk Control**: Detect problems and adjust direction in time

### Core Design Decisions

- **Bytecode First**: Implement interpretation first, then AOT
- **Progressive Delivery**: Each version has available functionality
- **Backward Compatible**: APIs may change before v1.0, but will announce in advance
- **Bootstrap Verification**: Prove language expressiveness through bootstrapping
- **Performance Layering**: Get it working first, then optimize

## 2. Component Status (Phase)

| Phase | Module | Status | Location | Last Update |
|-------|--------|--------|----------|-------------|
| P1 | Lexer | ✅ Done | `src/frontend/lexer/` | 2025-01-23 |
| P2 | Type Checker | ✅ Done | `src/frontend/typecheck/` | 2025-01-23 |
| P3 | Bytecode Generator | ✅ Done | `src/middle/codegen/` | 2025-01-25 |
| P4 | Virtual Machine | ✅ Done | `src/middle/` | 2025-01-25 |
| P4.1 | Task System | ✅ Done | `src/backends/runtime/task.rs` | 2025-01-23 |
| P4.2 | DAG Scheduler | 🔶 Design Done | `.claude/plan/flow-scheduler-implementation.md` | 2026-01-04 |
| P5 | Standard Library | ⚠️ Partial | `src/std/` | 2025-01-23 |
| P6 | TUI REPL | ✅ Done | `src/backends/dev/repl/` | 2025-01-24 |
| P7 | Generic System | ✅ Done | `docs/design/rfc/011-generic-type-system.md` | 2025-01-25 |

**Core Achievements**:
- ✅ Complete compiler frontend implementation (P1-P2)
- ✅ Bytecode generation and virtual machine completed (P3-P4)
- ✅ Basic task system completed (P4.1)
- ✅ TUI REPL development completed (P6)
- ✅ Generic system design completed (P7)

**Next Priority**: Implement FlowScheduler → Complete Standard Library (P5) → v0.1 Release

## 3. Version Roadmap

### v0.1: Runnable Milestone ✅

**Status**: Basic completion (2025-01-25)

**Completed**:
- ✅ Complete lexical analysis, syntax analysis, type checking
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
- Complete task system implementation
- Modern TUI REPL interface
- Unified type syntax + generic system design

**Not Included**: Complete DAG scheduling (basic scheduler already implemented)

### v0.2: FlowScheduler Scheduler 🚧

**Goal**: Implement complete dependency-aware scheduler

- ✅ Design documentation completed
- 🔶 Implementation in progress
- [ ] DAG nodes and graph implementation
- [ ] Work stealing algorithm
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

### v0.4: Generic System 📋

**Goal**: Complete generic capability

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

- IO, dictionary, network modules
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
- Toolchain completion
- Edge case fixes

### v0.9: Bootstrap Start 📋

**Goal**: Core modules rewritten in YaoXiang

- Lexer → Parser → TypeChecker → Codegen gradually replace
- Cross-validation: two compilers produce same results

### v1.0: Production Ready 📋

**Goal**: Stable release

- Complete bootstrap
- AOT compilation (LLVM backend)
- Production ready

## 4. Three-Layer Compilation Strategy Design

| Layer | Version | Input | Output | Description |
|-------|---------|-------|--------|-------------|
| L1: Bytecode | v0.1+ | Source (.yx) | Bytecode (.yxb) | VM interpretation |
| L2: Bootstrap | v0.9+ | YaoXiang Source | Bytecode | Compile self |
| L3: AOT | v1.0+ | Source/Bytecode | Machine Code | Native performance |

**Reasons for bytecode first**:
1. **REPL Support**: Immediately compile input code, interactive development
2. **Progressive Compilation**: Modifying single function only needs recompiling that part
3. **Platform Independence**: .yxb files run cross-platform, only need VM for each platform

## 5. Dependency Strategy

**Short-term**: Call Rust libraries and reuse crates.io (Cargo parasitism)

**Current Dependencies**:
- Concurrency: parking_lot, crossbeam, rayon
- Data structures: indexmap, hashbrown, smallvec
- Network: tokio
- Serialization: serde, ron

**Long-term**: Build standard library and package manager

## 6. Toolchain

| Version | Tool | Status |
|---------|------|--------|
| v0.1 | yaoxiang-cli | ✅ Done |
| v0.1 | TUI REPL | ✅ Done |
| v0.2 | yaoxiang-debug | 🚧 In Design |
| v0.3 | yaoxiang-fmt | 📋 Planned |
| v0.3 | yaoxiang-lsp (basic) | 📋 Planned |
| v0.5 | yaoxiang-clippy | 📋 Planned |
| v1.0 | Complete toolchain | 📋 Planned |

## 7. Success Metrics

| Metric | v0.1 | v0.2 | v0.3 | v0.5 | v1.0 |
|--------|------|------|------|------|------|
| End-to-end run | ✅ | ✅ | ✅ | ✅ | ✅ |
| Basic task system | ✅ | ✅ | ✅ | ✅ | ✅ |
| FlowScheduler | ❌ | 🚧 | ✅ | ✅ | ✅ |
| Concurrency support | ⚠️ | 🚧 | ✅ Basic | ✅ Complete | ✅ |
| Standard library | Basic | Basic | Basic | Complete | Complete |
| Generic system | ⚠️ | ⚠️ | 🚧 | ✅ | ✅ |
| TUI REPL | ✅ | ✅ | ✅ | ✅ | ✅ |
| Bootstrap | ❌ | ❌ | ❌ | ❌ | ✅ |
| AOT | ❌ | ❌ | ❌ | ❌ | ✅ |
| Code coverage | 60% | 70% | 80% | 90% | 95% |

**Legend**:
- ✅ Done
- 🚧 In Progress
- ⚠️ Partial
- 📋 Planned

## 8. Open Questions

- [ ] JIT vs AOT timing selection
- [ ] Package manager design
- [ ] Bootstrap module replacement order
- [ ] AOT backend selection (LLVM vs custom)

## 9. Version Release Standards

**v0.x Series**:
- Feature complete but may have edge case issues
- APIs may change
- For learning and experimentation only

**v1.0**:
- All core features stable
- API frozen
- Suitable for production use
- Complete documentation and tutorials

## References

- [Semantic Versioning 2.0.0](https://semver.org/)
- [Rust Release Model](https://forge.rust-lang.org/release.html)
- [RFC-001: Concurrency Model and Error Handling](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model](./008-runtime-concurrency-model.md)
