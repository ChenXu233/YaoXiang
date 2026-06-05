---
title: "Runtime State"
---

# Runtime

> **Module Status**: Gaps remaining (4 items pending improvement, Phase B not started)
> **Location**: `src/backends/runtime/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The runtime module is responsible for task scheduling and concurrent execution. Implements the three-tier runtime architecture defined in RFC-008: Embedded / Standard / Full.

**Code Size**: ~95KB (4 source files)

---

## Feature List

### engine.rs — DAG Scheduling Core (Implemented)

- ✅ **Task Lifecycle Management**: spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG Dependency Graph**: Hard dependencies (hard_deps, propagating failure/cancellation) + Control dependencies (control_deps, ordering only without failure propagation)
- ✅ **Resource Serialization**: ResourceKey mechanism, tasks with the same key are strictly serialized, waiting on control dependencies upon cancellation to maintain resource order
- ✅ **Cycle Detection**: Detects cycles via reachability during add_dependency, returns CycleDetected error
- ✅ **Failure/Cancellation Propagation**: Failures propagate BFS along hard dependency edges, when multiple dependencies fail simultaneously, cancellation reasons are merged (primary + others)
- ✅ **Cooperative Time Slicing**: drive_until_polled supports TaskPoll::Pending yielding, fair rotation among multiple tasks
- ✅ **Target-First Scheduling**: next_ready_for(target) prioritizes advancing the target dependency chain, island tasks don't block target completion
- ✅ **Statistics**: RuntimeStats (pending/running/completed/failed/cancelled/total_spawned/avg_execution_time)

### facade.rs — Three-Tier Runtime Facade (Implemented)

- ✅ **Embedded Runtime**: Immediate execution, runs closures immediately on spawn, no DAG, no support for deps/resources
- ✅ **Standard Runtime**: Single-threaded DAG scheduling, supports regular TaskFn and cooperative CoopTaskFn
- ✅ **Full Runtime**: Multi-threaded execution, crossbeam channel communication, worker thread pool
- ✅ **Unified Facade Runtime**: Configured via RuntimeConfig(mode, workers, work_stealing)

### task.rs — Task Abstraction (Implemented)

- ✅ **TaskId**: Unique identifier, supports Display
- ✅ **TaskPriority**: Low/Normal/High/Critical four levels
- ✅ **TaskState**: Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**: Builder pattern configuration (priority/name/stack_size/parent_id)
- ✅ **Task**: Task entity, holds id/config/state/result
- ✅ **TaskContext**: Task execution context (registers/stack/locals/entry_ip)
- ✅ **Scheduler trait**: Abstract scheduler interface
- ✅ **TaskSpawner**: Generic task spawner wrapper

---

## Test Coverage

**~22 unit tests**, covering core scenarios:

| Test File | Test Count | Covered Scenarios |
|-----------|------------|-------------------|
| `engine.rs` | 14 | Linear dependencies, diamond dependencies, island tasks, target scheduling, failure propagation, cancellation, resource serialization, cycle detection, cooperative slicing |
| `facade.rs` | 5 | Standard/Full consistency, parallel execution, resource serialization, work-stealing toggle, cooperative slicing |
| `task.rs` | 3 | TaskId, TaskConfig, TaskContext |

---

## RFC Comparison (RFC-008)

| RFC-008 Requirement | Implementation Status | Notes |
|---------------------|----------------------|-------|
| Three-tier architecture Embedded/Standard/Full | ✅ Implemented | Three RuntimeInner variants in facade.rs |
| Scheduler decoupling (generics + injection) | ⚠️ Partially implemented | Scheduler trait exists in task.rs, but facade.rs uses enum directly |
| Sync = special case of scheduling (num_workers=1) | ✅ Implemented | Verified by Full workers=1 test matching Standard behavior |
| DAG lazy evaluation | ✅ Implemented | LocalRuntime in engine.rs |
| Bottom-up execution model | ✅ Implemented | drive_until / next_ready_for prioritizes target dependency chain |
| Island DAGs execute independently in parallel without blocking | ✅ Implemented | Dedicated test exists |
| WorkStealer | ⚠️ Declared support but not actually implemented | FullRuntime uses crossbeam channel, no real work-stealing queue |
| Compile-time DAG analysis | ❌ Not implemented (Phase B) | Current DAG is built at runtime |
| Scheduler static library (200-500KB) | ❌ Not implemented (Phase B) | Falls under LLVM AOT compiler scope |
| On-demand reflection metadata loading | ❌ Not implemented (Phase B) | Belongs to future roadmap |

---

## Key Findings

1. **The Scheduler trait in task.rs is separate from the actual scheduling in facade.rs**: task.rs defines the Scheduler trait, but facade.rs doesn't use this trait; it dispatches directly via enum
2. **task.rs has duplicate type definitions**: SyncValue, TaskResult, RuntimeError, SchedulerStats are each defined in both engine.rs and task.rs
3. **WorkStealing is not actually implemented**: RuntimeConfig has a work_stealing field, but FullRuntime is actually a simple thread pool + channel model

---

## Code Quality Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Unfinished items | 4 | WorkStealing, duplicate types, unified scheduler, Phase B |
| Test coverage | Good | 22 tests covering core scenarios |
| Documentation quality | Good | Complete module-level and method-level documentation |
| Code architecture | Excellent | Clear three-tier architecture, good separation of concerns |
| RFC compliance | Highly compliant | All Phase A acceptance criteria are checked |

---

## Areas for Improvement

1. **Implement a real WorkStealing queue**
2. **Eliminate duplicate type definitions in task.rs**
3. **Unify Scheduler trait with facade.rs scheduling implementation**
4. **Begin Phase B: Compiler integration**