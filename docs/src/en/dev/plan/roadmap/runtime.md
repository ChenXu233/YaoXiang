---
title: "Runtime State"
---

# Runtime

> **Module Status**: Gaps present (4 items pending improvement, Phase B not started)
> **Location**: `src/backends/runtime/`
> **Last Updated**: 2026-06-05

---

## Module Overview

The runtime module is responsible for task scheduling and concurrent execution. Implements the three-layer runtime architecture defined in RFC-008: Embedded / Standard / Full.

**Code Size**: ~95KB (4 source files)

---

## Feature Checklist

### engine.rs — DAG Scheduling Core (Implemented)

- ✅ **Task lifecycle management**: spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG dependency graph**: hard dependencies (hard_deps, propagate failure/cancellation) + control dependencies (control_deps, ordering only, no failure propagation)
- ✅ **Resource serialization**: ResourceKey mechanism, tasks with same key are strictly serialized; on cancellation, waits for control dependencies to maintain resource order
- ✅ **Circular dependency detection**: Cycle detection via reachability in add_dependency, returns CycleDetected error
- ✅ **Failure/cancellation propagation**: Failures propagate along hard dependency edges via BFS; when multiple dependencies fail simultaneously, cancellation reasons are merged (primary + others)
- ✅ **Cooperative time-slicing**: drive_until_polled supports TaskPoll::Pending for yielding, fair round-robin among multiple tasks
- ✅ **Target-first scheduling**: next_ready_for(target) prioritizes advancing the target dependency chain; island tasks do not block target completion
- ✅ **Statistics**: RuntimeStats (pending/running/completed/failed/cancelled/total_spawned/avg_execution_time)

### facade.rs — Three-Layer Runtime Facade (Implemented)

- ✅ **Embedded Runtime**: Immediate execution; closures run immediately on spawn, no DAG, no deps/resources support
- ✅ **Standard Runtime**: Single-threaded DAG scheduling, supports both regular TaskFn and cooperative CoopTaskFn
- ✅ **Full Runtime**: Multi-threaded execution, crossbeam channel communication, worker thread pool
- ✅ **Unified Runtime facade**: Configured via RuntimeConfig(mode, workers, work_stealing)

### task.rs — Task Abstraction (Implemented)

- ✅ **TaskId**: Unique identifier, implements Display
- ✅ **TaskPriority**: Four levels - Low/Normal/High/Critical
- ✅ **TaskState**: Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**: Builder pattern for configuration (priority/name/stack_size/parent_id)
- ✅ **Task**: Task entity, holds id/config/state/result
- ✅ **TaskContext**: Task execution context (registers/stack/locals/entry_ip)
- ✅ **Scheduler trait**: Abstract scheduler interface
- ✅ **TaskSpawner**: Generic task scheduler wrapper

---

## Test Coverage

**~22 unit tests**, covering core scenarios:

| Test File | Test Count | Covered Scenarios |
|-----------|------------|-------------------|
| `engine.rs` | 14 | Linear dependencies, diamond dependencies, island tasks, target scheduling, failure propagation, cancellation, resource serialization, cycle detection, cooperative slicing |
| `facade.rs` | 5 | Standard/Full consistency, parallel execution, resource serialization, work-stealing toggle, cooperative slicing |
| `task.rs` | 3 | TaskId, TaskConfig, TaskContext |

---

## RFC Comparison (RFC-008 / RFC-024)

| RFC Requirement | Implementation Status | Notes |
|-----------------|----------------------|-------|
| Three-layer architecture Embedded/Standard/Full | ✅ Implemented | Three RuntimeInner variants in facade.rs |
| Scheduler decoupling (generics + injection) | ⚠️ Partially implemented | Scheduler trait exists in task.rs, but facade.rs directly uses enum dispatch |
| Synchronous = special case of scheduling (num_workers=1) | ✅ Implemented | Full workers=1 test verifies consistency with Standard |
| Lazy DAG evaluation | ✅ Implemented | LocalRuntime in engine.rs |
| Bottom-up execution model | ✅ Implemented | drive_until / next_ready_for prioritize target dependency chain |
| Island DAG independent parallelism does not block | ✅ Implemented | Dedicated tests exist |
| WorkStealer | ⚠️ Declared but not independently implemented | FullRuntime uses crossbeam channel, no real work-stealing queue |
| Compile-time DAG analysis | ❌ Not implemented (RFC-024 Phase B) | Current DAG built at runtime; RFC-024 plans to move to compile-time |
| Spawn block direct sub-expression parallelism | ❌ Not implemented (RFC-024) | Current spawn wraps entire block as single closure |
| Static scheduler library (200-500KB) | ❌ Not implemented (Phase B) | Falls under LLVM AOT compiler scope |
| Reflection metadata on-demand loading | ❌ Not implemented (Phase B) | Falls under future planning |

---

## Key Findings

1. **Scheduler trait in task.rs is decoupled from actual scheduling in facade.rs**: task.rs defines the Scheduler trait, but facade.rs does not use this trait; instead it directly dispatches via enum
2. **Duplicate type definitions in task.rs**: SyncValue, TaskResult, RuntimeError, SchedulerStats are each defined in both engine.rs and task.rs
3. **WorkStealing not truly implemented**: RuntimeConfig has work_stealing field, but FullRuntime is actually a simple thread pool + channel model
4. **RFC-024 will change the spawn execution model**: Current spawn wraps entire block as single closure for runtime DAG scheduling; RFC-024 plans to analyze dependencies of direct sub-expressions within spawn blocks at compile-time, generate execution plan, and runtime executes groups in parallel according to plan

---

## Code Quality Assessment

| Dimension | Rating | Notes |
|-----------|--------|-------|
| Incomplete Items | 4 | WorkStealing, duplicate types, unified scheduler, Phase B |
| Test Coverage | Good | 22 tests covering core scenarios |
| Documentation Quality | Good | Module-level and method-level documentation complete |
| Code Architecture | Excellent | Three-layer architecture clear, separation of concerns good |
| RFC Compliance | Highly compliant | All Phase A acceptance criteria checked |

---

## Items to Improve

1. **Implement true WorkStealing queue**
2. **Eliminate duplicate type definitions in task.rs**
3. **Unify Scheduler trait with facade.rs scheduling implementation**
4. **RFC-024 Phase B: Compiler integration**
   - Clean up old model (remove `@block`/`@eager`/`@auto`, EvalMode, EvalStrategy)
   - Add compile-time DAG analysis pass
   - Modify `Instruction::Spawn` to support multiple closures + execution plan
   - Runtime executes groups in parallel according to compile-time execution plan