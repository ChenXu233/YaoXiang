---
title: "Borrow Checker Status"
---

# Borrow Checker (Lifetime)

> **Module Status**: Transition Period — v8 Linear Scan Architecture → v9 Hoare Proposition Pipeline
> **Location**: `src/middle/passes/lifetime/`
> **Last Updated**: 2026-06-13
>
> **Related RFCs**:
> - [RFC-009: Ownership Model Design](../design/rfc/accepted/009-ownership-model.md) — Accepted
> - [RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline Based](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — Accepted

---

## Module Overview

The Borrow Checker module is responsible for YaoXiang's ownership analysis — Move semantics, borrow token conflict detection, Drop/Clone correctness, ref cycle detection, and mutability violation checks.

**Current Architecture** (Transition Period):
- ir_gen hardcodes the insertion of Borrow/Release instructions (lexical scope)
- BorrowChecker linearly scans IR, passively validating token conflicts
- ControlFlowAnalyzer exists but its core logic is empty
- User-visible behavior is largely correct, but the underlying implementation is not the Hoare proposition pipeline from RFC-009a

**Target Architecture** (RFC-009 + RFC-009a):
- Brand tree tracks token derivation relationships
- Consumer analysis drives NLL (Non-Lexical Lifetimes) release
- Reverse BFS liveness analysis (fast path covers 95%+ of scenarios)
- SMT logic cutoff as fallback (for the extremely rare while + path condition scenarios)
- Release is driven by scope analysis, not hardcoded in ir_gen

**Code Volume**: approximately 300KB of source code (15 sub-files)

---

## RFC Alignment Status

### RFC-009 Five Core Concepts

| Concept | User-Visible Behavior | Underlying Implementation |
|---------|----------------------|---------------------------|
| **Move** | ✅ Completed | MoveChecker, UseAfterMove detection |
| **&T / &mut T** | ✅ Completed | BorrowChecker linear scan, passively responds to Borrow/Release instructions |
| **ref** | ⚠️ Cycle detection done, escape analysis missing | ref_semantics + cycle_check + intra_task_cycle |
| **clone()** | ✅ Completed | CloneChecker, 0 tests |
| **unsafe + *T** | ✅ Completed | UnsafeChecker |

### RFC-009a Six Phases

| Phase | Content | Status | Description |
|-------|---------|--------|-------------|
| 1 | Brand tree data structure | ❌ Not started | Replaces HashMap<String, BorrowToken>, brand ID + parent node + consumer list |
| 2 | Consumer analysis | ❌ Not started | Consumers automatically collected during DAG construction, NLL foundation |
| 3 | Reverse BFS liveness analysis | ❌ Not started | Brand tree + consumers + break cutoff → covers 95%+ of scenarios |
| 4 | Scope-driven Release | ❌ Not started | Remove ir_gen hardcoding, LIFO insertion at scope exit points, automatic `?` handling |
| 5 | SMT logic cutoff | ❌ Not started | Blocked on RFC-027 Phase 2, only triggered by while + path conditions |
| 6 | Cleanup | ❌ Not started | BorrowChecker → BorrowPredicateEmitter, delete ControlFlowAnalyzer |

---

## Current Module Inventory

### Core Checkers

| Sub-module | File | Function | Tests |
|------------|------|----------|-------|
| **Move Semantics** | `move_semantics.rs` | UseAfterMove detection, empty-state reassignment | 6 |
| **Drop Semantics** | `drop_semantics.rs` | UseAfterDrop, DropMovedValue, DoubleDrop | 0 |
| **Mutability Check** | `mut_check.rs` | Immutable variable assignment / mutating methods / field assignment | 0 |
| **Ref Semantics** | `ref_semantics.rs` | RefNonOwner detection | 0 |
| **Clone Semantics** | `clone.rs` | CloneMovedValue, CloneDroppedValue | 0 |
| **Borrow Tokens** | `borrow_checker.rs` | Token conflict detection (linear scan architecture) | 16 |
| **Cross-Task Cycles** | `cycle_check.rs` | Cross-spawn loop reference DFS detection | 8 |
| **Intra-Task Cycles** | `intra_task_cycle.rs` | Intra-task ref cycle tracking (warning mode) | 7 |

### Auxiliary Modules

| Sub-module | File | Destination |
|------------|------|-------------|
| **Ownership Reflow** | `ownership_flow.rs` | Keep |
| **Consume Analysis** | `consume_analysis.rs` | → Phase 2, integrated into brand tree |
| **Chained Calls** | `chain_calls.rs` | Keep |
| **Lifecycle Tracking** | `lifecycle.rs` | Keep — Drop insertion requires it |
| **Empty State** | `empty_state.rs` | Keep |
| **Control Flow** | `control_flow.rs` | → Phase 6, delete |
| **Unsafe Check** | `unsafe_check.rs` | Keep |
| **Send/Sync** | `send_sync.rs` | Keep (used independently) |

---

## Implementation Roadmap

### Phase 0: Fill in Tests (Can start immediately, blocks refactoring)

> Before touching the architecture, lay down the test net for existing behavior.

| # | Task | File |
|---|------|------|
| 0.1 | Add Drop semantics tests | `tests/drop_semantics.rs` |
| 0.2 | Add Clone semantics tests | `tests/clone.rs` |
| 0.3 | Add mutability check tests | `tests/mut_check.rs` |
| 0.4 | Add Ref semantics tests | `tests/ref_semantics.rs` |
| 0.5 | Add Unsafe check tests | `tests/unsafe_check.rs` |

### Phase 1: Brand Tree Data Structure (RFC-009a Phase 1)

| # | Task | Output |
|---|------|--------|
| 1.1 | Define `BrandTree`, `BrandNode` structs | `brand_tree.rs` |
| 1.2 | Implement prefix matching conflict judgment | `conflicts()` |
| 1.3 | Implement brand node registration during DAG construction | Integrate into ir_gen |
| 1.4 | Unit tests | `tests/brand_tree.rs` |

### Phase 2: Consumer Analysis (RFC-009a Phase 2)

| # | Task | Output |
|---|------|--------|
| 2.1 | Automatically collect consumer list for each token during DAG construction | `BrandNode.consumers` |
| 2.2 | System predicate generator definition (Borrow/Move/Drop/Mut → `{P} op {Q}`) | Interface definition |

### Phase 3: Reverse BFS Liveness Analysis (RFC-009a Phase 3)

| # | Task | Output |
|---|------|--------|
| 3.1 | Implement reverse BFS algorithm (break cuts back-edges) | Fast path |
| 3.2 | Hook into RFC-027 proof pipeline interface (Proved/Disproved) | Pipeline integration |
| 3.3 | NLL iteration boundary rule implementation | Cross-iteration token semantics inside loops |
| 3.4 | Replace BorrowChecker linear scan | Remove the Borrow/Release match in `check_instruction` |

### Phase 4: Scope-Driven Release (RFC-009a Phases 4-5)

| # | Task | Output |
|---|------|--------|
| 4.1 | Scope exit point collection (`}`, `?`, explicit return) | ir_gen |
| 4.2 | LIFO Release insertion (parent-child relationships in brand tree auto-cascade) | ir_gen |
| 4.3 | Remove hardcoded Release after Call in `ir_gen.rs` | Code cleanup |

### Phase 5: SMT Logic Cutoff (RFC-009a Phase 5, depends on RFC-027 Phase 2)

| # | Task | Output |
|---|------|--------|
| 5.1 | Path condition collection integration | Get from RFC-027 pipeline |
| 5.2 | SMT fallback: `path_cond ⇒ !loop_cond` | Slow path |
| 5.3 | Activate borrow checking inside while loop bodies | Currently conservatively rejected scenarios |

### Phase 6: Cleanup (RFC-009a Phase 6)

| # | Task | Output |
|---|------|--------|
| 6.1 | `BorrowChecker` → `BorrowPredicateEmitter` | Rename, clarify responsibilities |
| 6.2 | Delete `ControlFlowAnalyzer` | Unified handling by the pipeline |
| 6.3 | Migrate `consume_analysis.rs` consumer information into brand tree | De-duplication |
| 6.4 | Update error message format | Align with RFC-009a §Error Message Design |

---

## Independent Tasks (Do not block the main track)

| # | Task | Description |
|---|------|-------------|
| I.1 | ref escape analysis (automatic Rc vs Arc selection) | The current compiler does not distinguish between cross-task and intra-task, uses Arc uniformly |
| I.2 | Before deleting `ControlFlowAnalyzer` in `control_flow.rs`, do not add new code to it | — |

---

## Test Coverage

**Currently: 83 unit tests**

| File | Test Count | Coverage |
|------|------------|----------|
| `borrow_checker.rs` | 16 | Sufficient |
| `chain_calls.rs` | 13 | Sufficient |
| `consume_analysis.rs` | 11 | Sufficient |
| `ownership_flow.rs` | 10 | Sufficient |
| `lifecycle.rs` | 10 | Sufficient |
| `cycle_check.rs` | 8 | Good |
| `intra_task_cycle.rs` | 7 | Good |
| `move_semantics.rs` | 6 | Basic |
| `control_flow.rs` | 1 | Insufficient |
| `empty_state.rs` | 1 | Insufficient |
| Others | 0 | **Missing**: drop_semantics, clone, mut_check, ref_semantics, unsafe_check |

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Outstanding Items | 10 | Phase 0 tests (5) + Phases 1-6 architecture (6) + ref escape analysis (1) |
| Test Coverage | To be strengthened | 5 sub-modules with 0 tests, must be filled in before refactoring |
| Documentation Quality | Good | Module/struct/method-level documentation comments are all present |
| Code Architecture | Transition Period | The current linear scan architecture works but does not align with RFC-009a |