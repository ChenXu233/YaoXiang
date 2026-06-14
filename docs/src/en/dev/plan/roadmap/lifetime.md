---
title: "Borrow Checker Status"
---

# Borrow Checker (Lifetime)

> **Module Status**: Stable (4 items pending)
> **Location**: `src/middle/passes/lifetime/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The borrow checker module is a complete **ownership analysis and lifetime management system**, responsible for checking ownership-related issues such as Move semantics, borrow conflicts, and mutability violations.

**Code Volume**: ~300KB source code (15 subfiles)

---

## Feature List

### Core Checkers (Integrated into the OwnershipChecker unified entry point)

| Submodule | File | Function | Status |
|--------|------|----------|--------|
| **Move Semantics** | `move_semantics.rs` (575 lines) | UseAfterMove detection, supports empty state (Empty) reassignment | ✅ Completed |
| **Drop Semantics** | `drop_semantics.rs` (143 lines) | UseAfterDrop, DropMovedValue, DoubleDrop detection | ✅ Completed |
| **Mutability Check** | `mut_check.rs` (395 lines) | Immutable variable assignment, immutable object mutation method, immutable field assignment | ✅ Completed |
| **Ref Semantics** | `ref_semantics.rs` (145 lines) | RefNonOwner detection — ref can only be applied to valid owners | ✅ Completed |
| **Clone Semantics** | `clone.rs` (173 lines) | CloneMovedValue, CloneDroppedValue detection | ✅ Completed |
| **Borrow Token** | `borrow_checker.rs` (503 lines) | Borrow token conflict detection: MutableBorrowConflict, BorrowAfterMove, UseWhileFrozen | ✅ Completed |
| **Cross-spawn Loop** | `cycle_check.rs` (616 lines) | Cross-task circular reference detection, DFS cycle detection | ✅ Completed |
| **Intra-task Cycle** | `intra_task_cycle.rs` (406 lines) | Intra-task ref cycle tracking (warning mode) | ✅ Completed |

### Auxiliary Analyzers

| Submodule | File | Function | Status |
|--------|------|----------|--------|
| **Ownership Flow** | `ownership_flow.rs` (426 lines) | Analyzes whether function parameters are returned in return values | ✅ Completed |
| **Consume Analysis** | `consume_analysis.rs` (363 lines) | Cross-function consume pattern queries, supports caching | ✅ Completed |
| **Chain Calls** | `chain_calls.rs` (652 lines) | Method chain ownership flow analysis | ✅ Completed |
| **Lifetime Tracking** | `lifecycle.rs` (1037 lines) | Complete variable lifetime tracking | ✅ Completed |
| **Empty State** | `empty_state.rs` (513 lines) | Variable empty state tracking after Move | ✅ Completed |
| **Control Flow** | `control_flow.rs` (353 lines) | Branch state merging analysis | ⚠️ Skeleton complete, core analysis logic is an empty implementation |
| **Unsafe Check** | `unsafe_check.rs` (113 lines) | Report error for dereferencing raw pointer outside unsafe block | ✅ Completed |
| **Send/Sync** | `send_sync.rs` (401 lines) | Type-level Send/Sync constraint check and constraint propagation | ✅ Completed (used independently) |

---

## Test Coverage

**83 unit tests**, distributed as follows:

| File | Test Count | Coverage Status |
|------|-----------|-----------------|
| `borrow_checker.rs` | 16 | Most thorough: unit tests + end-to-end tests |
| `chain_calls.rs` | 13 | Thorough: chain extraction, consume pattern inference, long chains, mixed calls |
| `consume_analysis.rs` | 11 | Thorough: Returns/Consumes patterns, caching, multiple parameters |
| `ownership_flow.rs` | 10 | Thorough: direct return, indirect return, multiple parameters partial return |
| `lifecycle.rs` | 10 | Thorough: creation/consumption/release tracking, issue detection |
| `cycle_check.rs` | 8 | Good: no cycle / unidirectional chain / depth limit / unsafe bypass |
| `intra_task_cycle.rs` | 7 | Good: no cycle / simple cycle / self-reference / multiple cycles |
| `move_semantics.rs` | 6 | Basic: state tracking, UseAfterMove |
| `control_flow.rs` | 1 | Insufficient: only tests the state merging function |
| `empty_state.rs` | 1 | Insufficient: only tests state merging |
| Others | 0 | No tests: drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC Comparison (RFC-009 Ownership Model)

| RFC Design Point | Implementation Status | Notes |
|------------------|----------------------|-------|
| Move Semantics (default) | ✅ Implemented | MoveChecker detects UseAfterMove |
| &T/&mut T Borrow Token | ✅ Implemented | BorrowChecker implements token conflict detection |
| &T freezes source data (ReadToken) | ✅ Implemented | WriteToken is prohibited while ReadToken is alive; safe to Dup under the freeze guarantee |
| &mut T linear | ✅ Implemented | Only one &mut T from the same source can be active |
| Token conflict detection (flow-sensitive liveness analysis) | ✅ Implemented | Track token state within function body |
| ref keyword (Rc/Arc auto-selection) | ⚠️ Partially implemented | ref semantic checker exists |
| clone() explicit deep copy | ✅ Implemented | CloneChecker detects clone of moved/dropped values |
| unsafe + *T | ✅ Implemented | UnsafeChecker checks raw pointer operations outside unsafe block |
| Intra-task cycle: silently allowed | ✅ Implemented | IntraTaskCycleTracker tracks in warning mode |
| Cross-task cycle: lint | ✅ Implemented | CycleChecker detects circular references across spawn loops |
| No lifetime 'a | ✅ Complies with design | Lifetime parameters are not implemented |
| Send/Sync constraint | ✅ Implemented | SendSyncChecker is independent of OwnershipChecker |

---

## Code Quality Assessment

| Dimension | Rating | Notes |
|-----------|--------|-------|
| Pending items | 3 | Add tests, control_flow logic, ref escape analysis |
| Test coverage | Good | 83 tests, borrow_checker/chain_calls/consume_analysis are thoroughly tested |
| Documentation quality | Good | Documentation comments present at module/struct/method level |
| Code architecture | Excellent | OwnershipChecker unified orchestration, clear separation of concerns |
| RFC compliance | Highly compliant | RFC-009 v9 design is highly compliant |

---

## Items to Improve

1. **Add unit tests for 5 submodules**: drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **Implement the core logic of the control_flow analyzer** (currently an empty skeleton)
3. **Improve ref escape analysis for automatic Rc/Arc selection**