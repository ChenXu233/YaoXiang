---
title: 'RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design'
---

# RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2025-01-05
> **Last Updated**: 2025-01-25 (Update: Integrated RFC-011 generic system, type constraints defined)

> **Reference**:
> - [RFC-001: Concurrency Model and Error Handling System](../rfc/001-concurrent-model-error-handling.md)
> - [RFC-003: Version Planning and Implementation Suggestions](../rfc/003-version-planning.md)
> - [RFC-011: Generic Type System Design](../rfc/011-generic-type-system.md) **Type Constraints Defined**

## Summary

This document discusses key design issues in Runtime architecture:
1. **Runtime Layered Design**: Embedded Runtime vs Standard Runtime vs Full Runtime
2. **Compilation and Runtime Separation**: Compilation phase identical, difference only in runtime execution
3. **Scheduler Decoupling Design**: Through **generics + injection** achieve decoupling, ensuring interpreter still works when work stealing and other features not enabled
4. **YaoXiang Generics vs Rust Trait**: Use YaoXiang's generic system to directly implement scheduler decoupling, no need to introduce Trait concept at language level
5. **DAG's Core Position**: As lazy evaluation dependency graph, DAG belongs to Standard Runtime, embedded scenarios can choose not to use it

## Motivation

### Current Architecture Problems

When designing YaoXiang runtime architecture, the following key issues need resolution:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Current Architecture Confusion                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Problem 1: DAG - Core or Optional?                              │
│  ├── If core, should be in Core Runtime                         │
│  ├── If optional, how to implement "no concurrency" sync?        │
│  └── Lazy evaluation depends on DAG, cannot disable             │
│                                                                 │
│  Problem 2: How to decouple scheduler?                          │
│  ├── Can WorkStealer be disabled?                               │
│  ├── How to implement single-threaded mode?                     │
│  └── How to ensure VM still runs when "no scheduler features"?  │
│                                                                 │
│  Problem 3: Async/Concurrency implementation level              │
│  ├── @block (L1 sync)                                          │
│  ├── spawn (L2 explicit concurrency)                            │
│  └── Unmarked (L3 transparent concurrency)                      │
│  └── Where should these features be implemented?                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Core Contradictions

| Contradiction | Description |
|--------------|-------------|
| Transparency vs Controllability | Concurrency should be default, but user should control |
| Core vs Optional | DAG is core, but WorkStealing is advanced feature for num_workers>1 |
| Single-threaded vs Concurrency | Single-threaded concurrency is async, sync is just scheduling special case |

## Proposal

### 1. Runtime Three-Layer Architecture Design

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                           YaoXiang Runtime Three-Layer Architecture                     │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                       │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐   │
│  │                           📦 Compilation Phase (Same for All Modes)                │   │
│  │                                                                                  │   │
│  │   Source Code                                                                    │   │
│  │       │                                                                          │   │
│  │       ▼                                                                          │   │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐  │   │
│  │   │  Lexer  │─▶│ Parser  │─▶│TypeCheck│─▶│ Codegen │─▶│  IR / Bytecode      │  │   │
│  │   └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────────────┘  │   │
│  │                                                                                  │   │
│  └──────────────────────────────────────────────────────────────────────────────────┘   │
│                                        │                                               │
│                                        ▼                                               │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐   │
│  │                           📦 Runtime Selection (Decoupled)                         │   │
│  │                                                                                  │   │
│  │   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐   │   │
│  │   │  Embedded RT     │  │  Standard RT     │  │  Full RT                     │   │   │
│  │   │  (Embedded)     │  │  (Default)      │  │  (Advanced)                  │   │   │
│  │   ├─────────────────┤  ├─────────────────┤  ├─────────────────────────────┤   │   │
│  │   │ • Tiny          │  │ • Full          │  │ • WorkStealing             │   │   │
│  │   │ • No GC         │  │ • GC Optional   │  │ • Advanced Scheduling      │   │   │
│  │   │ • No DAG        │  │ • DAG          │  │ • Full Profiling          │   │   │
│  │   │ • Sync only     │  │ • @block       │  │ • @auto optimization      │   │   │
│  │   └─────────────────┘  └─────────────────┘  └─────────────────────────────┘   │   │
│  │                                                                                  │   │
│  └──────────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                       │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### 2. Runtime Layer Definitions

| Runtime | Use Case | Features |
|--------|----------|----------|
| **Embedded** | Microcontrollers, No-OS | Minimal footprint, sync execution only |
| **Standard** | General purpose | Full features, DAG support |
| **Full** | High performance | Work stealing, profiling |

### 3. Scheduler Decoupling Design

#### Generic Scheduler Trait

```yaoxiang
# Scheduler trait (using YaoXiang generics)
type Scheduler[T: Task] = {
    schedule: (T) -> Void,
    spawn: (T) -> Void,
    wait: (T) -> Void,
    yield: () -> Void,
}
```

#### Default Scheduler Implementations

```yaoxiang
# Single-threaded scheduler
type SingleThreadedScheduler = {
    schedule: (Task) -> Void,
    spawn: (Task) -> Void,
    wait: (Task) -> Void,
    yield: () -> Void,
}

# Work-stealing scheduler
type WorkStealingScheduler = {
    schedule: (Task) -> Void,
    spawn: (Task) -> Void,
    wait: (Task) -> Void,
    yield: () -> Void,
    steal: () -> Task,
}
```

### 4. DAG Integration

#### DAG as Standard Runtime Feature

```yaoxiang
# DAG builder (Standard Runtime)
type DAG = {
    add_node: (Task) -> NodeId,
    add_edge: (NodeId, NodeId) -> Void,
    topological_sort: () -> List[NodeId],
    execute: () -> Result,
}
```

#### DAG Disabled for Embedded

```yaoxiang
# No DAG in Embedded Runtime
# Sync execution only, no dependency tracking
```

### 5. Concurrency Annotations

| Annotation | Runtime | Behavior |
|------------|---------|----------|
| `@block` | All | No DAG, sync execution |
| `@eager` | Standard+ | Build DAG, eager evaluation |
| `@auto` | Standard+ | Build DAG, lazy evaluation |

## Implementation

### Phase 1: Runtime Abstraction

| Task | Status |
|------|--------|
| Scheduler trait definition | ✅ |
| Runtime interface | ✅ |
| Dependency injection | 🔄 |

### Phase 2: Scheduler Implementations

| Scheduler | Status |
|-----------|--------|
| Single-threaded | 🔄 |
| Work-stealing | ⏳ |
| Distributed | ⏳ |

### Phase 3: DAG Integration

| Feature | Status |
|---------|--------|
| DAG builder | 🔄 |
| Topological sort | ⏳ |
| Lazy evaluation | ⏳ |

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Three-layer runtime | Embedded/Standard/Full | 2025-01-05 | ChenXu |
| Scheduler decoupling | Generics + injection | 2025-01-05 | ChenXu |
| DAG in Standard | Not in Embedded | 2025-01-25 | ChenXu |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| DAG | Directed Acyclic Graph for dependency tracking |
| Scheduler | Component that manages task execution |
| Work Stealing | Load balancing technique |
| Runtime | Execution environment |
| Embedded Runtime | Minimal runtime for constrained environments |
