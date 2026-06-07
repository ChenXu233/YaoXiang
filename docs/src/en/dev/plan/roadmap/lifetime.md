---
title: "Borrow Checker Status"
---

# Borrow Checker (Lifetime)

> **Module Status**: Stable (4 items pending improvement)
> **Location**: `src/middle/passes/lifetime/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The Borrow Checker module is a complete **ownership analysis and lifetime management system**, responsible for checking Move semantics, borrow conflicts, mutability violations, and other ownership-related issues.

**Code Size**: ~300KB source code (15 sub-files)

---

## Feature List

### Core Checkers (Integrated into OwnershipChecker unified entry point)

| Sub-module | File | Functionality | Status |
|------------|------|---------------|--------|
| **Move Semantics** | `move_semantics.rs` (575 lines) | UseAfterMove detection, supports reassignment of Empty state | ✅ Complete |
| **Drop Semantics** | `drop_semantics.rs` (143 lines) | UseAfterDrop, DropMovedValue, DoubleDrop detection | ✅ Complete |
| **Mutability Check** | `mut_check.rs` (395 lines) | Immutable variable assignment, immutable object mutation methods, immutable field assignment | ✅ Complete |
| **Ref Semantics** | `ref_semantics.rs` (145 lines) | RefNonOwner detection — ref can only be applied to valid owners | ✅ Complete |
| **Clone Semantics** | `clone.rs` (173 lines) | CloneMovedValue, CloneDroppedValue detection | ✅ Complete |
| **Borrow Token** | `borrow_checker.rs` (503 lines) | Borrow token conflict detection: MutableBorrowConflict, BorrowAfterMove, UseWhileFrozen | ✅ Complete |
| **Cross-spawn Loop** | `cycle_check.rs` (616 lines) | Cross-task circular reference detection, DFS cycle detection | ✅ Complete |
| **Intra-task Loop** | `intra_task_cycle.rs` (406 lines) | Intra-task ref cycle tracking (warning mode) | ✅ Complete |

### Auxiliary Analyzers

| Sub-module | File | Functionality | Status |
|------------|------|---------------|--------|
| **Ownership Flow** | `ownership_flow.rs` (426 lines) | Analyzes whether function parameters are returned in return values | ✅ Complete |
| **Consume Analysis** | `consume_analysis.rs` (363 lines) | Cross-function consume pattern queries, supports caching | ✅ Complete |
| **Chain Calls** | `chain_calls.rs` (652 lines) | Method chain ownership flow analysis | ✅ Complete |
| **Lifecycle Tracking** | `lifecycle.rs` (1037 lines) | Complete variable lifecycle tracking | ✅ Complete |
| **Empty State** | `empty_state.rs` (513 lines) | Variable empty state tracking after Move | ✅ Complete |
| **Control Flow** | `control_flow.rs` (353 lines) | Branch state merge analysis | ⚠️ Skeleton complete, core analysis logic is empty implementation |
| **Unsafe Check** | `unsafe_check.rs` (113 lines) | Report errors for dereferencing raw pointers outside unsafe blocks | ✅ Complete |
| **Send/Sync** | `send_sync.rs` (401 lines) | Type-level Send/Sync constraint checking and constraint propagation | ✅ Complete (standalone use) |

---

## Test Coverage

**83 unit tests**, distributed as follows:

| File | Test Count | Coverage |
|------|------------|----------|
| `borrow_checker.rs` | 16 | Most thorough: unit tests + end-to-end tests |
| `chain_calls.rs` | 13 | Thorough: chain extraction, consume pattern inference, long chains, mixed calls |
| `consume_analysis.rs` | 11 | Thorough: Returns/Consumes patterns, caching, multi-parameter |
| `ownership_flow.rs` | 10 | Thorough: direct return, indirect return, multi-parameter partial return |
| `lifecycle.rs` | 10 | Thorough: create/consume/release tracking, issue detection |
| `cycle_check.rs` | 8 | Good: no cycles/one-way chain/depth limit/unsafe bypass |
| `intra_task_cycle.rs` | 7 | Good: no cycles/simple cycles/self-reference/multiple cycles |
| `move_semantics.rs` | 6 | Basic: state tracking, UseAfterMove |
| `control_flow.rs` | 1 | Insufficient: only tests state merge function |
| `empty_state.rs` | 1 | Insufficient: only tests state merge |
| Others | 0 | No tests: drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC Comparison (RFC-009 Ownership Model)

| RFC Design Point | Implementation Status | Description |
|------------------|----------------------|-------------|
| Move semantics (default) | ✅ Implemented | MoveChecker detects UseAfterMove |
| &T/&mut T borrow token | ✅ Implemented | BorrowChecker implements token conflict detection |
| &T copyable (Dup) | ✅ Implemented | Multiple &T tokens can coexist |
| &mut T linear | ✅ Implemented | Only one active &mut T from the same source at a time |
| Token conflict detection (flow-sensitive liveness analysis) | ✅ Implemented | Tracks token state within function body |
| ref keyword (auto-select Rc/Arc) | ⚠️ Partially implemented | ref semantics checker exists |
| clone() explicit deep copy | ✅ Implemented | CloneChecker detects cloned moved/released values |
| unsafe + *T | ✅ Implemented | UnsafeChecker checks raw pointer operations outside unsafe blocks |
| Intra-task cycles: silently allowed | ✅ Implemented | IntraTaskCycleTracker tracks in warning mode |
| Cross-task cycles: lint | ✅ Implemented | CycleChecker detects cross-spawn circular references |
| No lifetime 'a | ✅ Design compliant | Lifetime parameters not implemented |
| Send/Sync constraints | ✅ Implemented | SendSyncChecker independent of OwnershipChecker |

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Incomplete items | 3 | Supplement tests, control_flow logic, ref escape analysis |
| Test coverage | Good | 83 tests, borrow_checker/chain_calls/consume_analysis well tested |
| Documentation quality | Good | Module/struct/method level all have doc comments |
| Code architecture | Excellent | OwnershipChecker unified orchestration, clear separation of concerns |
| RFC compliance | Highly compliant | Highly aligned with RFC-009 v9 design |

---

## Pending Improvements

1. **Add unit tests for 5 sub-modules**: drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **Implement core logic of control_flow analyzer** (currently an empty skeleton)
3. **Complete ref auto-select Rc/Arc escape analysis**