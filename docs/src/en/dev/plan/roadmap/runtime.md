---
title: "Runtime State"
---

# Runtime

> **Module Status**: Gaps remain (4 items pending, Phase B not started)
> **Location**: `src/backends/runtime/`
> **Last Updated**: 2026-06-05

---

## Module Overview

The runtime module is responsible for task scheduling and concurrent execution. It implements the three-tier runtime architecture defined in RFC-008: Embedded / Standard / Full.

**Code Size**: ~95KB (4 source files)

---

## Feature List

### engine.rs — DAG Scheduling Core (Implemented)

- ✅ **Task Lifecycle Management**: spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG Dependency Graph**: hard dependencies (hard_deps, propagate failure/cancellation) + control dependencies (control_deps, ordering only, no failure propagation)
- ✅ **Resource Serialization**: ResourceKey mechanism, tasks with the same key run strictly serially, waits on control dependencies during cancellation to maintain resource ordering
- ✅ **Cycle Detection**: detects cycles via reachability check during add_dependency, returns CycleDetected error
- ✅ **Failure/Cancellation Propagation**: failures propagate along hard dependency edges via BFS, merges cancellation reasons when multiple dependencies fail (primary + others)
- ✅ **Cooperative Time Slicing**: drive_until_polled supports TaskPoll::Pending yield, fair round-robin across multiple tasks
- ✅ **Target-Priority Scheduling**: next_ready_for(target) prioritizes advancing the target's dependency chain, orphan tasks don't block target completion
- ✅ **Statistics**: RuntimeStats (pending/running/completed/failed/cancelled/total_spawned/avg_execution_time)

### facade.rs — Three-Tier Runtime Facade (Implemented)

- ✅ **Embedded Runtime**: immediate execution, runs closures immediately on spawn, no DAG, no deps/resources support
- ✅ **Standard Runtime**: single-threaded DAG scheduling, supports regular TaskFn and cooperative CoopTaskFn
- ✅ **Full Runtime**: multi-threaded execution, crossbeam channel communication, worker thread pool
- ✅ **Unified Runtime Facade**: configured via RuntimeConfig(mode, workers, work_stealing)

### task.rs — Task Abstraction (Implemented)

- ✅ **TaskId**: unique identifier, supports Display
- ✅ **TaskPriority**: four levels — Low/Normal/High/Critical
- ✅ **TaskState**: Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**: builder pattern configuration (priority/name/stack_size/parent_id)
- ✅ **Task**: task entity, holding id/config/state/result
- ✅ **TaskContext**: task execution context (registers/stack/locals/entry_ip)
- ✅ **Scheduler trait**: abstract scheduler interface
- ✅ **TaskSpawner**: generic task scheduler wrapper

---

## Test Coverage

**~22 unit tests** covering core scenarios:

| Test File | Test Count | Coverage |
|-----------|-----------|----------|
| `engine.rs` | 14 | Linear dependencies, diamond dependencies, orphan tasks, target scheduling, failure propagation, cancellation, resource serialization, cycle detection, cooperative slicing |
| `facade.rs` | 5 | Standard/Full consistency, parallel execution, resource serialization, work-stealing toggle, cooperative slicing |
| `task.rs` | 3 | TaskId, TaskConfig, TaskContext |

---

## RFC Comparison (RFC-008 / RFC-024)

| RFC Requirement | Implementation Status | Notes |
|-----------------|----------------------|-------|
| Three-tier architecture Embedded/Standard/Full | ✅ Implemented | Three RuntimeInner variants in facade.rs |
| Scheduler decoupling (generics + injection) | ⚠️ Partially implemented | Scheduler trait exists in task.rs, but facade.rs uses enum directly |
| Synchronous = special case of scheduling (num_workers=1) | ✅ Implemented | Full with workers=1 test verifies consistency with Standard |
| Lazy DAG evaluation | ✅ Implemented | LocalRuntime in engine.rs |
| Bottom-up execution model | ✅ Implemented | drive_until / next_ready_for prioritizes target dependency chain |
| Orphan DAG runs in parallel without blocking | ✅ Implemented | Has dedicated tests |
| WorkStealer | ⚠️ Declared supported but not actually implemented independently | FullRuntime uses crossbeam channel, no real work-stealing queue |
| Compile-time DAG analysis | ❌ Not implemented (RFC-024 Phase B) | DAG currently built at runtime; RFC-024 plans to move to compile-time |
| spawn block direct sub-expression parallelism | ❌ Not implemented (RFC-024) | spawn currently wraps the whole block as a single closure |
| Scheduler static library (200-500KB) | ❌ Not implemented (Phase B) | Falls under LLVM AOT compiler scope |
| On-demand reflection metadata loading | ❌ Not implemented (Phase B) | Part of future plans |

---

## Key Findings

1. **The Scheduler trait in task.rs is separated from the actual scheduling in facade.rs**: task.rs defines the Scheduler trait, but facade.rs does not use this trait, dispatching directly via enum instead.
2. **task.rs has duplicate type definitions**: SyncValue, TaskResult, RuntimeError, SchedulerStats are defined in both engine.rs and task.rs.
3. **WorkStealing not actually implemented**: RuntimeConfig has a work_stealing field, but FullRuntime is actually a simple thread pool + channel model.
4. **RFC-024 will change spawn's execution model**: currently spawn wraps the whole block as a single closure scheduled by the runtime DAG; RFC-024 plans to analyze the dependency relationships between direct sub-expressions within spawn blocks at compile-time, generate an execution plan, and have the runtime execute in parallel groups according to the plan.

---

## Code Quality Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Outstanding items | 4 | WorkStealing, duplicate types, unified scheduler, Phase B |
| Test coverage | Good | 22 tests cover core scenarios |
| Documentation quality | Good | Module-level and method-level documentation complete |
| Code architecture | Excellent | Three-tier architecture is clear, with good separation of concerns |
| RFC compliance | Highly compliant | All Phase A acceptance criteria are checked |

---

## Pending Improvements

1. **Implement a real WorkStealing queue**
2. **Eliminate duplicate type definitions in task.rs**
3. **Unify the Scheduler trait with the scheduling implementation in facade.rs**
4. **RFC-024 Phase B: Compiler Integration**
   - ~~Clean up old model~~ ✅ Done (`@block`/`@eager`/`@auto`, `Send`/`Sync` removed)
   - Add a compile-time DAG analysis pass
   - Modify `Instruction::Spawn` to support multiple closures + execution plan
   - Runtime executes in parallel groups according to the compile-time execution plan