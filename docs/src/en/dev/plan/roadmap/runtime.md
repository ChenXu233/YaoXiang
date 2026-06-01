---
title: "Runtime State"
---

# Runtime

> **Module Status**: Completed (Stage A), Stage B partially not started
> **Location**: `src/backends/runtime/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The runtime module is responsible for task scheduling and concurrent execution. It implements the three-layer runtime architecture defined in RFC-008: Embedded / Standard / Full.

**Code Size**: ~95KB (4 source files)

---

## Feature List

### engine.rs — DAG Scheduling Core (Implemented)

- ✅ **Task lifecycle management**: spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG dependency graph**: hard dependencies (hard_deps, propagate failure/cancel) + control dependencies (control_deps, ordering only, no failure propagation)
- ✅ **Resource serialization**: ResourceKey mechanism, tasks with same key are strictly serialized; on cancel, wait for control dependencies to maintain resource order
- ✅ **Circular dependency detection**: cycle detection via reachability during add_dependency, returns CycleDetected error
- ✅ **Failure/cancel propagation**: failure propagates along hard dependency edges via BFS; when multiple dependencies fail simultaneously, merge cancel reasons (primary + others)
- ✅ **Cooperative time slicing**: drive_until_polled supports TaskPoll::Pending yielding; fair round-robin for multiple tasks
- ✅ **Target-first scheduling**: next_ready_for(target) prioritizes advancing the target dependency chain; island tasks do not block target completion
- ✅ **Statistics**: RuntimeStats (pending/running/completed/failed/cancelled/total_spawned/avg_execution_time)

### facade.rs — Three-Layer Runtime Facade (Implemented)

- ✅ **Embedded Runtime**: immediate execution, closures run immediately on spawn, no DAG, no deps/resources support
- ✅ **Standard Runtime**: single-threaded DAG scheduling, supports regular TaskFn and cooperative CoopTaskFn
- ✅ **Full Runtime**: multi-threaded execution, crossbeam channel communication, worker thread pool
- ✅ **Unified Facade Runtime**: configured via RuntimeConfig(mode, workers, work_stealing)

### task.rs — Task Abstraction (Implemented)

- ✅ **TaskId**: unique identifier, supports Display
- ✅ **TaskPriority**: Low/Normal/High/Critical four levels
- ✅ **TaskState**: Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**: builder pattern configuration (priority/name/stack_size/parent_id)
- ✅ **Task**: task entity, holds id/config/state/result
- ✅ **TaskContext**: task execution context (registers/stack/locals/entry_ip)
- ✅ **Scheduler trait**: abstract scheduler interface
- ✅ **TaskSpawner**: generic task scheduler wrapper

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
| Three-layer architecture Embedded/Standard/Full | ✅ Implemented | Three RuntimeInner types in facade.rs |
| Scheduler decoupling (generics + injection) | ⚠️ Partially implemented | Scheduler trait exists in task.rs, but facade.rs uses enum directly |
| Sync = special case of scheduling (num_workers=1) | ✅ Implemented | Full workers=1 test verifies consistency with Standard |
| Lazy DAG evaluation | ✅ Implemented | LocalRuntime in engine.rs |
| Bottom-up execution model | ✅ Implemented | drive_until / next_ready_for prioritize target dependency chain |
| Island DAG independent parallel, no blocking | ✅ Implemented | Dedicated tests exist |
| WorkStealer | ⚠️ Declared but not independently implemented | FullRuntime uses crossbeam channel, no real work-stealing queue |
| Compile-time DAG analysis | ❌ Not implemented (Stage B) | Current DAG is built at runtime |
| Scheduler static library (200-500KB) | ❌ Not implemented (Stage B) | Belongs to LLVM AOT compiler scope |
| Reflection metadata on-demand loading | ❌ Not implemented (Stage B) | Part of future planning |

---

## Key Findings

1. **Scheduler trait in task.rs is decoupled from facade.rs actual scheduling**: task.rs defines the Scheduler trait, but facade.rs does not use this trait; it dispatches directly via enum
2. **task.rs has duplicate type definitions**: SyncValue, TaskResult, RuntimeError, SchedulerStats are each defined in both engine.rs and task.rs
3. **WorkStealing not truly implemented**: RuntimeConfig has a work_stealing field, but FullRuntime is actually a simple thread pool + channel model

---

## Code Quality Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Feature completeness | 80% | Stage A fully complete, Stage B not started |
| Test coverage | Good | 22 tests cover core scenarios |
| Documentation quality | Good | Module-level and method-level documentation complete |
| Code architecture | Excellent | Three-layer architecture clear, good separation of concerns |
| RFC compliance | Highly compliant | All acceptance criteria for Stage A are checked |

---

## Items for Improvement

1. **Implement a real WorkStealing queue**
2. **Eliminate duplicate type definitions in task.rs**
3. **Unify Scheduler trait with facade.rs scheduling implementation**
4. **Start Stage B: compiler integration**