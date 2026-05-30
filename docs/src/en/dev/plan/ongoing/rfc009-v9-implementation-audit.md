```markdown
---
title: RFC-009 v9 Implementation Completeness Audit Report
status: ongoing
created: 2026-05-29
---

# RFC-009 v9 Implementation Completeness Audit Report

## Audit Scope

Cross-referencing the RFC-009 v9 Borrow Token System design document, examining the completeness of compiler implementation on an item-by-item basis. Covers six dimensions: type system, parser, borrow checker, IR generation, Dup system, and closure capture.

---

## 1. Frontend (Type System + Parser)

| # | Check Item | Status | File | Description |
|---|------------|--------|------|-------------|
| 1.1 | Lexer `&`/`&mut` | ✅ | `tokenizer.rs` L249-268, `tokens.rs` L75-76 | Two TokenKind variants exist: `Ampersand` and `MutRef`; `&&` (And) unaffected |
| 1.2 | AST `Type::Ref` | ✅ | `ast.rs` L474-480 | `Ref { mutable: bool, inner: Box<Type>, span: Span }` fields complete |
| 1.3 | AST `Expr::Borrow` | ✅ | `ast.rs` L131-137 | `Borrow { mutable: bool, expr: Box<Expr>, span: Span }` fields complete |
| 1.4 | Parser `&T`/`&mut T` type | ✅ | `types.rs` L73-93 | `parse_type_annotation` correctly matches `Ampersand` and `MutRef` |
| 1.5 | Parser `&expr`/`&mut expr` | ✅ | `nud.rs` L38-39, L196-213 | `parse_borrow` method correctly distinguishes mutable |
| 1.6 | MonoType `Ref { mutable, inner }` | ✅ | `mono.rs` L189-196 | Comment: "Compile-time zero-size type, no runtime representation" |
| 1.7 | MonoType Display | ✅ | `mono.rs` L342-348 | Outputs `&T` or `&mut T` |
| 1.8 | From<ast::Type> conversion | ✅ | `mono.rs` L556-559 | `Type::Ref` correctly converted to `MonoType::Ref` |
| 1.9 | Type checker Borrow inference | ✅ | `expressions.rs` L1096-1106 | Infers inner type and wraps as `MonoType::Ref` |
| 1.10 | Formatter | ✅ | `types.rs` L107-113, `expr.rs` L168-178 | `Type::Ref` and `Expr::Borrow` formatting correct |
| **1.11** | **Dup trait: `&T` is Dup, `&mut T` is not Dup** | **⚠️** | **`solver.rs` L201-233** | **`check_dup_trait` has no explicit match for `MonoType::Ref`; both fall into `_ => false`** |

---

## 2. Middle-end — Borrow Checker

| # | Check Item | Status | File | Description |
|---|------------|--------|------|-------------|
| 2.1 | BorrowChecker exists | ✅ | `borrow_checker.rs` | 496 lines, methods complete |
| 2.2 | Token state machine | ✅ | `borrow_checker.rs` | Three states: `Active`/`Frozen`/`Moved` |
| 2.3 | Multiple `&T` from same source allowed (Dup) | ✅ | `borrow_checker.rs` L174 | Immutable+immutable combination does not error |
| 2.4 | Creating `&T` while `&mut T` is active errors | ✅ | `borrow_checker.rs` L165-173 | Produces `MutableBorrowConflict` |
| 2.5 | Creating `&mut T` while `&mut T` is active errors | ✅ | `borrow_checker.rs` L147-155 | Produces `MutableBorrowConflict` |
| 2.6 | Using after freeze errors | ✅ | `borrow_checker.rs` L203-207 | Produces `UseWhileFrozen` |
| 2.7 | Using after move errors | ✅ | `borrow_checker.rs` L209-214 | Produces `BorrowAfterMove` |
| 2.8 | Freeze mechanism | ✅ | `borrow_checker.rs` | `&mut T` can freeze to `&T`; source `&mut` automatically unfreezes |
| 2.9 | OwnershipChecker integration | ✅ | `mod.rs` L122, L153-154 | `borrow_checker` field exists; called in `check_function` |
| 2.10 | Error types | ✅ | `error.rs` | `MutableBorrowConflict`/`BorrowAfterMove`/`UseWhileFrozen` three variants |
| **2.11** | **Brand mechanism (Brand)** | **❌** | **`borrow_checker.rs`** | **Only uses variable name strings for tracking origin; no compile-time unique ID; no derived brand chain** |
| **2.12** | **`&mut T` IR instruction** | **❌** | **`ir.rs`** | **No instruction for creating `&mut T` in IR; `create_borrow(mutable=true)` unreachable in IR analysis** |

---

## 3. Middle-end — IR Generation

| # | Check Item | Status | File | Description |
|---|------------|--------|------|-------------|
| **3.1** | **IR Borrow/Release instructions** | **❌** | **`ir.rs`** | **`Instruction` enum has no `Borrow` or `Release`; only `ArcNew`/`ArcClone`/`ArcDrop`** |
| **3.2** | **ir_gen Expr::Borrow handling** | **❌** | **`ir_gen.rs`** | **`generate_expr_ir` has no `Expr::Borrow` branch at all; `&expr` silently ignored at IR stage** |
| 3.3 | MakeClosure env population | ⚠️ | `ir_gen.rs` L3186-3201 | Capture analysis works, but missing ZST optimization (TODO comment) |
| 3.4 | Bytecode type_id | ⚠️ | `bytecode.rs` L413 | type_id 49 allocated, but no corresponding instruction |
| 3.5 | Bytecode From<MonoType> | ⚠️ | `bytecode.rs` L1418-1424 | Placeholder stub; all types mapped to `IrType::Void` |
| **3.6** | **Interpreter Borrow handling** | **❌** | **`execute.rs`** | **No borrow-related handling; `RuntimeValue` has no borrow variant** |
| **3.7** | **ZST optimization** | **❌** | **`ir_gen.rs`** | **`MonoType::Ref` annotated as ZST, but IR generation has no optimization logic** |

---

## 4. Dup System (RFC-011 Section 2.4)

| # | Check Item | Status | File | Description |
|---|------------|--------|------|-------------|
| 4.1 | Dup trait registration | ✅ | `std_traits.rs` L31 | `"Dup"` in `STD_TRAITS` |
| 4.2 | is_marker field | ✅ | `trait_data.rs` L31 | `pub is_marker: bool` exists |
| 4.3 | Dup implies Clone | ✅ | `std_traits.rs` L115 | `parent_traits: vec!["Clone"]` |
| 4.4 | Primitive Dup impl | ✅ | `std_traits.rs` L153-186 | Int/Float/Bool/Char/String/Bytes |
| 4.5 | Solver recursive check | ✅ | `solver.rs` L201-233 | Recursive Struct/Enum/Tuple fields |
| 4.6 | Auto-derive generic support | ✅ | `auto_derive.rs` | `Type::Generic`/`Type::Tuple` recursive handling |
| 4.7 | Bounds integration | ✅ | `bounds.rs` L51-58 | Falls back to auto-derive on failure |
| **4.8** | **Send/Sync residual** | **⚠️** | **`solver.rs`, `auto_derive.rs`, `send_sync.rs`** | **`check_send_trait`/`check_sync_trait` methods residual; `BUILTIN_DERIVES` contains Send/Sync; `send_sync.rs` still exported with `pub mod`** |

---

## 5. Closure Capture (RFC-023)

| # | Check Item | Status | File | Description |
|---|------------|--------|------|-------------|
| 5.1 | capture.rs module | ✅ | `capture.rs` | ~1065 lines, with tests |
| 5.2 | Capture analysis | ✅ | `capture.rs` L133-170 | Traverses Lambda body, distinguishes Read/Write |
| 5.3 | Escape analysis | ✅ | `capture.rs` L83-123 | Spawn/Return/Assignment → Escaping |
| 5.4 | Pattern selection | ✅ | `capture.rs` L186-206 | Dup→Copy, Escaping→Move, Inline→Borrow/BorrowMut |
| 5.5 | Integration into type checking | ✅ | `expressions.rs` L934-968 | Capture analysis called during Lambda inference |
| 5.6 | MakeClosure env population | ✅ | `ir_gen.rs` L3177-3243 | `env: Vec::new()` replaced with actual captured variables |

---

## Issue List (Sorted by Priority)

### P0 — Blocking Issues (Borrow tokens cannot work at runtime)

| # | Issue | Location | Impact | Fix Direction |
|---|-------|----------|--------|---------------|
| P0-1 | IR has no Borrow/Release instructions | `ir.rs` | Borrow tokens cannot be represented in IR | Add new `Borrow { dst, src, mutable }` and `Release { src }` instructions |
| P0-2 | Expr::Borrow IR generation missing | `ir_gen.rs` | `&expr` silently ignored at IR stage | Add Borrow branch in `generate_expr_ir`, generate Borrow instruction |
| P0-3 | Interpreter has no Borrow handling | `execute.rs` | Cannot execute borrow operations at runtime | Add `BytecodeInstr::Borrow`/`Release` handling |

### P1 — Important Issues (Semantic Correctness)

| # | Issue | Location | Impact | Fix Direction |
|---|-------|----------|--------|---------------|
| P1-1 | `&T` does not satisfy Dup | `solver.rs` L201-233 | Violates RFC core semantics: "&T can be freely copied" | Add `MonoType::Ref { mutable: false, .. } => true` branch |
| P1-2 | Brand mechanism missing | `borrow_checker.rs` | Cannot track token derivation relationships | Add compile-time unique ID and derived brand chain |
| P1-3 | `&mut T` IR instruction unreachable | `borrow_checker.rs` | Mutable borrow conflict detection never triggers | Automatically resolved after adding MutRef instructions at IR layer |

### P2 — Improvement Issues (Code Quality)

| # | Issue | Location | Impact | Fix Direction |
|---|-------|----------|--------|---------------|
| P2-1 | MakeClosure ZST optimization | `ir_gen.rs` L3196-3198 | Token capture produces meaningless overhead | ZST types skip env |
| P2-2 | Send/Sync residual | `solver.rs`, `auto_derive.rs`, `send_sync.rs` | Code residue | Delete `check_send_trait`/`check_sync_trait`, clean up `BUILTIN_DERIVES` |
| P2-3 | Bytecode From<MonoType> placeholder | `bytecode.rs` L1418-1424 | All types mapped to Void | Implement real type conversion |

---

## Implementation Status Overview

```
Frontend (Type System + Parser)    ████████████████████░  91% (10/11)
Middle-end (Borrow Checker)        ██████████████░░░░░░░  75% (9/12)
Middle-end (IR Generation)         ░░░░░░░░░░░░░░░░░░░░   0% (0/3 core)
Dup System                         ██████████████████░░  88% (7/8)
Closure Capture                    ████████████████████ 100% (6/6)
```

**Overall completion: ~65%**. Frontend complete; middle-end borrow checker logic complete but IR layer is broken.

---

## Recommended Implementation Order

1. **P1-1**: Add `MonoType::Ref` Dup match in `solver.rs` (1-line change)
2. **P2-2**: Clean up Send/Sync residual code
3. **P0-1**: Add Borrow/Release IR instructions in `ir.rs`
4. **P0-2**: Add `Expr::Borrow` IR generation in `ir_gen.rs`
5. **P0-3**: Add interpreter Borrow handling in `execute.rs`
6. **P2-1**: MakeClosure ZST optimization
7. **P1-2**: Brand mechanism (can be deferred; current variable name tracking sufficient for basic functionality)
```