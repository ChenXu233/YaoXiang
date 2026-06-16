---
title: "Ownership Checker Status"
---

# Ownership Checker

> **Module Status**: Migration complete — frontend Hoare proposition pipeline has taken over
> **New Architecture Location**: `src/frontend/core/typecheck/layers/ownership.rs` (~1600 lines)
> **Legacy Location**: `src/middle/passes/lifetime/` (retained, gradually being cleaned up)
> **Last Updated**: 2026-06-15
>
> **Related RFCs**:
> - [RFC-009: Ownership Model Design](../design/rfc/accepted/009-ownership-model.md) — Accepted
> - [RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — Accepted
>
> **Known Issues**: [ongoing/ownership-known-issues.md](../ongoing/ownership-known-issues.md) — 6 defects and precision tradeoffs

---

## Module Overview

The Ownership Checker is responsible for YaoXiang's ownership analysis — Move semantics, borrow token conflict detection, Drop correctness, mutability violations, NLL (Non-Lexical Lifetimes) precise release, closure capture, function signature queries, and ref escape analysis.

**Current Architecture** (v9 Hoare proposition pipeline):
- Brand tree (BrandTree) tracking token derivation relationships and conflict judgments
- Consumer analysis driving NLL release (ReleasePlan)
- Reverse BFS liveness analysis (fast path, covering 95%+ of scenarios)
- SMT logical cutoff fallback (extremely rare while + path-condition scenarios)
- Scope-driven Drop (automatically marking `VarState::Dropped` when leaving scope)
- Closure capture analysis (save/restore/diff → CapturesStore)
- Function signature queries (TypeEnvironment → T/&T/&mut T → Move/ReadBorrow/WriteBorrow)
- ref escape analysis (use within `spawn` → Arc, otherwise → Rc)
- ir_gen reads ReleasePlan to insert Drop instructions + selects RcNew/ArcNew based on escaped_refs

---

## RFC Alignment Status

### RFC-009 Five Core Concepts

| Concept | User-Visible Behavior | Underlying Implementation |
|------|-------------|---------|
| **Move** | ✅ Completed | OwnershipChecker, UseAfterMove detection |
| **&T / &mut T** | ✅ Completed | Brand tree token conflict detection (fast path + SMT fallback) |
| **ref** | ✅ Completed | Escape analysis automatically selects Rc/Arc |
| **clone()** | ✅ Completed | CloneChecker, 0 tests |
| **unsafe + *T** | ✅ Completed | UnsafeChecker |

### RFC-009a Six Stages (New Version — Frontend Implementation)

| Stage | Content | Status | Notes |
|------|------|------|------|
| 1 | Brand tree data structure | ✅ Completed | `BrandTree` + `BrandNode` + `BrandId` |
| 2 | Consumer analysis | ✅ Completed | `BrandNode.consumers`, automatically collected via AST traversal |
| 3 | Reverse BFS liveness analysis | ✅ Completed | `fast_path_check()`, break cuts off back edges |
| 4 | Scope-driven Release | ✅ Completed | `ReleasePlan` + `scope_vars` stack, LIFO Drop |
| 5 | SMT logical cutoff | ✅ Completed | `smt_cut(path_cond, loop_cond)` via Z3 |
| 6 | Cleanup | ✅ Completed | Legacy files deleted, error code format not unified (P2) |

### Supplementary Stages

| Stage | Content | Status | Notes |
|------|------|------|------|
| D.1 | ref escape analysis (Rc vs Arc) | ✅ Completed | `ref_vars` + `escaped_refs` + `inside_spawn`, ref attribute propagation |
| D.2 | Test coverage expansion | ✅ Completed | 61 tests (original 31 → target 50+) |
| D.3 | Drop semantic trigger points | ✅ Completed | `VarState::Dropped` activated, automatically marked on scope exit |
| D.4 | Mutability check | ✅ Completed | `&mut` and assignment check `var_mutability`, emit `mut_violation` |
| D.5 | Roadmap sync | ✅ Completed | This document |
| — | Closure capture analysis | ✅ Completed | save→walk→diff→restore→CapturesStore |
| — | Function signature query | ✅ Completed | TypeEnvironment.get_var → T/&T/&mut T |
| — | Spawn walk | ✅ Completed | save/restore prevents polluting outer scope, detects ref escape |

---

## New Architecture Core Components

### `src/frontend/core/typecheck/layers/ownership.rs` (~1600 lines)

| Component | Function |
|------|------|
| `BrandId` / `BrandTree` | Token identifier + derivation tree + conflict judgment + consumer tracking |
| `ControlFlowGraph` | CFG nodes/edges/path conditions, Break/BackEdge |
| `fast_path_check()` | Reverse BFS liveness analysis (fast path) |
| `smt_cut()` | SMT logical cutoff (slow path, Z3) |
| 5 system predicates | borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation |
| `OwnershipChecker` | AST traversal + brand tree + CFG + predicate verification |
| `ReleasePlan` | NLL precise release plan (consumer + scope Drop dual-source merge) |
| `VarState` | Alive / Moved / Dropped three-state |
| `Captures` / `CapturesStore` | Closure captured variable set + storage |
| `StateSnapshot` | save_state / restore_state / diff_captures |
| `ParamOwnership` | Move / ReadBorrow / WriteBorrow |
| `ref_vars` / `escaped_refs` / `inside_spawn` | ref escape analysis (with attribute propagation) |

### `src/middle/core/ir_gen.rs`

- Reads `TypeCheckResult.release_plan` → inserts `Drop` instructions (NLL precise release points)
- Reads `TypeCheckResult.escaped_refs` → `Expr::Ref` selects `RcNew` or `ArcNew`

### `src/middle/core/ir.rs` / `bytecode.rs` / `opcode.rs`

- New `RcNew` instruction + `Opcode::RcNew(0x89)`

---

## Current Middle Layer Module List

> Note: `borrow_checker.rs`, `control_flow.rs`, `consume_analysis.rs`, `move_semantics.rs`,
> `drop_semantics.rs`, `mut_check.rs`, `ref_semantics.rs`, `clone.rs`, `empty_state.rs`,
> `send_sync.rs` have been deleted. The following are the retained active modules.

| Submodule | File | Function |
|--------|------|------|
| **Chain calls** | `chain_calls.rs` | Chained method call analysis |
| **Cross-task cycle** | `cycle_check.rs` | Cross-spawn reference cycle DFS |
| **Intra-task cycle** | `intra_task_cycle.rs` | Intra-task ref cycle tracking |
| **Lifecycle** | `lifecycle.rs` | IR-level Drop position tracking |
| **Ownership flow** | `ownership_flow.rs` | Function ownership flow analysis |
| **Unsafe** | `unsafe_check.rs` | unsafe block bypass check |
| **Error type** | `error.rs` | ValueState + Checker trait |

---

## Test Coverage

**Frontend Ownership Checker: 61 unit tests**

| Test Category | Test Count | Coverage |
|----------|--------|----------|
| Basics (BrandId/conflict/cascade/consumer/fast path) | 17 | Token prefix, conflict judgment, cascade deletion, consumer tracking, BFS liveness |
| System predicates | 6 | borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation |
| E2E integration (basic) | 7 | use after move, valid move, argument move, borrow conflict, write-write conflict, read-read safe |
| E2E mutability | 5 | &mut non-mut, &mut mut, assign non-mut, assign mut, parameter non-mut |
| E2E Drop | 2 | Scope Drop (ReleasePlan), nested block Drop |
| E2E Move+Borrow | 1 | borrow detection after move |
| E2E control flow | 2 | if/else dual-branch conflict, borrow inside while loop |
| E2E Drop ordering | 1 | Multi-variable same-span release |
| E2E return value | 2 | return Move, use after return |
| E2E multiple borrow | 2 | Three ReadTokens, Read+Write conflict |
| E2E block expression | 1 | Inner block variable scope |
| E2E consecutive Move | 2 | Consecutive Move, double move detection |
| E2E parameters | 2 | Parameter move then use, parameter not in ReleasePlan |
| E2E closure capture | 5 | Move capture, Read capture, no capture, defined-then-called, second call |
| E2E function signature | 2 | Unknown function fallback Move, unregistered function fallback |
| E2E ref escape | 4 | No spawn no escape, escape inside spawn, non-ref no escape, nested spawn |

**Middle Layer Tests: 53 unit tests**

---

## Code Quality Assessment

| Dimension | Score | Notes |
|------|------|------|
| Outstanding items | 4 | Error message format unification (P2) + Stage 0 test completion (5) + 6 known issues |
| Test coverage | Good | Frontend 61 tests + middle 53 tests = 114 tests |
| Documentation quality | Good | Module/struct/method level documentation comments present |
| Code architecture | Migration complete | Frontend Hoare proposition pipeline has taken over core logic; middle layer legacy files deleted |