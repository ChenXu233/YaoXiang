---
title: "Borrow Checker Status"
---

# Borrow Checker (Lifetime)

> **Module Status**: Stable (4 items pending improvement)
> **Location**: `src/middle/passes/lifetime/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The Borrow Checker module is a complete **ownership analysis and lifecycle management system**, responsible for checking Move semantics, borrow conflicts, mutability violations, and other ownership-related issues.

**Code Volume**: ~300KB source code (15 sub-files)

---

## Feature List

### Core Checkers (Integrated into OwnershipChecker unified entry point)

| Sub-module | File | Functionality | Status |
|------------|------|---------------|--------|
| **Move Semantics** | `move_semantics.rs` (575 lines) | UseAfterMove detection, re-assignment support for empty state (Empty) | ✅ Completed |
| **Drop Semantics** | `drop_semantics.rs` (143 lines) | UseAfterDrop, DropMovedValue, DoubleDrop detection | ✅ Completed |
| **Mutability Check** | `mut_check.rs` (395 lines) | Immutable variable assignment, immutable object mutation method, immutable field assignment | ✅ Completed |
| **Ref Semantics** | `ref_semantics.rs` (145 lines) | RefNonOwner detection—ref can only be applied to valid owners | ✅ Completed |
| **Clone Semantics** | `clone.rs` (173 lines) | CloneMovedValue, CloneDroppedValue detection | ✅ Completed |
| **Borrow Tokens** | `borrow_checker.rs` (503 lines) | Borrow token conflict detection: MutableBorrowConflict, BorrowAfterMove, UseWhileFrozen | ✅ Completed |
| **Cross-spawn Loops** | `cycle_check.rs` (616 lines) | Cross-task loop reference detection, DFS cycle detection | ✅ Completed |
| **Intra-task Loops** | `intra_task_cycle.rs` (406 lines) | Intra-task ref loop tracking (warning mode) | ✅ Completed |

### Auxiliary Analyzers

| Sub-module | File | Functionality | Status |
|------------|------|---------------|--------|
| **Ownership Flow** | `ownership_flow.rs` (426 lines) | Analyze whether function parameters are returned in return values | ✅ Completed |
| **Consume Analysis** | `consume_analysis.rs` (363 lines) | Cross-function consume pattern queries, supports caching | ✅ Completed |
| **Chain Calls** | `chain_calls.rs` (652 lines) | Method chain ownership flow analysis | ✅ Completed |
| **Lifecycle Tracking** | `lifecycle.rs` (1037 lines) | Complete variable lifecycle tracking | ✅ Completed |
| **Empty State** | `empty_state.rs` (513 lines) | Move post variable empty state tracking | ✅ Completed |
| **Control Flow** | `control_flow.rs` (353 lines) | Branch state merge analysis | ⚠️ Skeleton completed, core analysis logic is empty implementation |
| **Unsafe Check** | `unsafe_check.rs` (113 lines) | Report errors for dereferencing raw pointers outside unsafe blocks | ✅ Completed |
| **Send/Sync** | `send_sync.rs` (401 lines) | Type-level Send/Sync constraint checking and constraint propagation | ✅ Completed (standalone usage) |

---

## Test Coverage

**83 unit tests**, distributed as follows:

| File | Test Count | Coverage |
|------|------------|----------|
| `borrow_checker.rs` | 16 | Most thorough: unit tests + end-to-end tests |
| `chain_calls.rs` | 13 | Thorough: chain extraction, consume pattern inference, long chains, mixed calls |
| `consume_analysis.rs` | 11 | Thorough: Returns/Consumes patterns, caching, multi-parameter |
| `ownership_flow.rs` | 10 | Thorough: direct return, indirect return, multi-parameter partial return |
| `lifecycle.rs` | 10 | Thorough: creation/consume/release tracking, problem detection |
| `cycle_check.rs` | 8 | Good: no cycles/single chains/depth limits/unsafe bypass |
| `intra_task_cycle.rs` | 7 | Good: no cycles/simple cycles/self-reference/multiple cycles |
| `move_semantics.rs` | 6 | Basic: state tracking, UseAfterMove |
| `control_flow.rs` | 1 | Insufficient: only tests state merge function |
| `empty_state.rs` | 1 | Insufficient: only tests state merge |
| Others | 0 | No tests: drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC Comparison (RFC-009 Ownership Model)

| RFC Design Point | Implementation Status | Notes |
|-----------------|----------------------|-------|
| Move semantics (default) | ✅ Implemented | MoveChecker detects UseAfterMove |
| &T/&mut T borrow tokens | ✅ Implemented | BorrowChecker implements token conflict detection |
| &T is copyable (Dup) | ✅ Implemented | Multiple &T tokens can coexist |
| &mut T is linear | ✅ Implemented | Only one &mut T from the same source can be active |
| Freeze mechanism | ✅ Implemented | freeze() freezes &mut T to &T |
| Token conflict detection (flow-sensitive liveness analysis) | ✅ Implemented | Tracks token state within function body |
| ref keyword (automatic Rc/Arc selection) | ⚠️ Partial implementation | ref semantics checker exists |
| clone() explicit deep copy | ✅ Implemented | CloneChecker detects clone of moved/dropped values |
| unsafe + *T | ✅ Implemented | UnsafeChecker checks raw pointer operations outside unsafe blocks |
| Intra-task loops: silently allowed | ✅ Implemented | IntraTaskCycleTracker tracks in warning mode |
| Cross-task loops: lint | ✅ Implemented | CycleChecker detects cross-spawn loop references |
| No lifetime 'a | ✅ Design compliant | No lifetime parameters implemented |
| Send/Sync constraints | ✅ Implemented | SendSyncChecker independent from OwnershipChecker |

---

## Code Quality Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Unfinished items | 4 | Supplementary tests, control_flow logic, freeze mechanism, ref escape analysis |
| Test coverage | Good | 83 tests, borrow_checker/chain_calls/consume_analysis thoroughly tested |
| Documentation quality | Good | Module/struct/method level documentation comments present |
| Code architecture | Excellent | OwnershipChecker unified orchestration, clear separation of responsibilities |
| RFC compliance | Highly compliant | RFC-009 v9 design highly compliant |

---

## Pending Improvements

1. **Add unit tests for 5 sub-modules**: drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **Implement core logic for control_flow analyzer** (currently empty skeleton)
3. **Complete freeze mechanism** (&mut T temporary freeze to &T)
4. **Complete ref automatic Rc/Arc selection escape analysis**