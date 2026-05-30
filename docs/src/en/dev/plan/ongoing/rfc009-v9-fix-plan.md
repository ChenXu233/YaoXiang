---
title: RFC-009 v9 Bug Fix Plan
status: ongoing
created: 2026-05-29
---

# RFC-009 v9 Bug Fix Plan

Based on the issue list from the [Audit Report](rfc009-v9-implementation-audit.md), provide specific fix plans for each item.

---

## Fix Order Overview

```
Phase 1 (Quick Fix, 1 Line Change):
  P1-1: solver.rs — &T Dup Matching

Phase 2 (Cleanup Residuals):
  P2-2: Send/Sync Residual Cleanup

Phase 3 (IR Layer Core, P0 Blocking Items):
  P0-1: ir.rs — Add Borrow/Release IR Instructions
  P0-2: ir_gen.rs — Expr::Borrow IR Generation
  P0-3: execute.rs — Interpreter Borrow Handling

Phase 4 (IR Layer Completion):
  P1-3: bytecode.rs — Borrow/Release Bytecode Instructions + Opcode
  P2-3: bytecode.rs — Real From<MonoType> Implementation

Phase 5 (Optimization Items):
  P2-1: ir_gen.rs — MakeClosure ZST Optimization
  P1-2: borrow_checker.rs — Brand Mechanism (Can Be Deferred)
```

---

## P1-1: solver.rs — `&T` Dup Matching (1 Line Change)

**Problem**: `check_dup_trait` does not explicitly match `MonoType::Ref`, so both `&T` and `&mut T` fall into `_ => false`. The RFC core semantics are: "`&T` can be freely copied (Dup), `&mut T` cannot."

**File**: `src/frontend/core/typecheck/traits/solver.rs` L201-233

**Current Code**:
```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool
        | MonoType::Char | MonoType::String | MonoType::Bytes
        | MonoType::Void => true,
        MonoType::Arc(_) => true,
        MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),
        MonoType::Struct(s) => s.fields.iter().all(|(_, ft)| self.check_dup_trait(ft)),
        MonoType::Enum(_) => true,
        _ => false,  // ← &T and &mut T both fall here
    }
}
```

**Fix**: Add one line after `MonoType::Arc(_)`:
```rust
// &T is a zero-size token, can be freely copied (Dup); &mut T cannot
MonoType::Ref { mutable: false, .. } => true,
```

**Complete Modified match**:
```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool
        | MonoType::Char | MonoType::String | MonoType::Bytes
        | MonoType::Void => true,
        MonoType::Arc(_) => true,
        // &T is a zero-size token, can be freely copied (Dup); &mut T cannot
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),
        MonoType::Struct(s) => s.fields.iter().all(|(_, ft)| self.check_dup_trait(ft)),
        MonoType::Enum(_) => true,
        _ => false,
    }
}
```

**Verification**: Existing tests + new test cases:
```rust
#[test]
fn test_ref_is_dup() {
    let solver = create_test_solver();
    let ref_ty = MonoType::Ref { mutable: false, inner: Box::new(MonoType::Int(IntType::I64)) };
    assert!(solver.check_dup_trait(&ref_ty), "&T should be Dup");
}

#[test]
fn test_mut_ref_is_not_dup() {
    let solver = create_test_solver();
    let mut_ref_ty = MonoType::Ref { mutable: true, inner: Box::new(MonoType::Int(IntType::I64)) };
    assert!(!solver.check_dup_trait(&mut_ref_ty), "&mut T should NOT be Dup");
}
```

**Impact**: Pure logic fix, no breaking changes. Solves the issue where `determine_capture_mode` incorrectly judges `&T` variables during closure capture analysis.

---

## P2-2: Send/Sync Residual Cleanup

**Problem**: `check_send_trait`/`check_sync_trait` methods, Send/Sync entries in `BUILTIN_DERIVES`, and the `send_sync.rs` module still exist.

**File List**:
1. `src/frontend/core/typecheck/traits/solver.rs` — Delete `check_send_trait`/`check_sync_trait` methods
2. `src/frontend/core/typecheck/traits/auto_derive.rs` — Remove Send/Sync entries from `BUILTIN_DERIVES`
3. `src/frontend/core/typecheck/traits/send_sync.rs` — Delete entire file
4. `src/frontend/core/typecheck/traits/mod.rs` — Remove `pub mod send_sync;`

**Verification**: `cargo build` compiles successfully + all tests pass.

---

## P0-1: ir.rs — Add Borrow/Release IR Instructions

**Problem**: `Instruction` enum has neither `Borrow` nor `Release`, borrow tokens cannot be represented in IR.

**File**: `src/middle/core/ir.rs`

**Design Plan**: Following the pattern of `ArcNew`/`ArcClone`/`ArcDrop`, add two instructions:

```rust
// =====================
// Borrow Token Instructions
// =====================
/// Create borrow token: dst = &src (immutable) or dst = &mut src (mutable)
/// Borrow token is zero-size type, disappears after compilation.
/// This instruction is only used for flow-sensitive analysis in the borrow checker.
/// Runtime is equivalent to Mov.
Borrow {
    dst: Operand,
    src: Operand,
    mutable: bool,
},
/// Release borrow token, end borrow lifetime
/// Borrow checker updates token state accordingly.
Release(Operand),
```

**Insertion Location**: After `ArcDrop(Operand)` (around L268), before `ShareRef`.

**Impact**: Requires synchronized updates to all `match` Instruction locations:
- IR → bytecode conversion in `ir_gen.rs`
- Instruction traversal in `borrow_checker.rs`

---

## P0-2: ir_gen.rs — `Expr::Borrow` IR Generation

**Problem**: `generate_expr_ir` has no `Expr::Borrow` branch, `&expr` is silently ignored at IR stage.

**File**: `src/middle/core/ir_gen.rs`

**Design Plan**: Add `Expr::Borrow` branch in the match of `generate_expr_ir`.

**Current Status**: `Expr::Borrow` is only matched in `get_expr_span` (L1996), used for getting span.

**Added Branch** (recommended insertion after `Expr::Lambda` branch):

```rust
ast::Expr::Borrow { mutable, expr, span } => {
    // 1. Generate IR for inner expression
    let inner_reg = self.next_temp_reg();
    self.generate_expr_ir(expr, inner_reg, instructions, constants)?;

    // 2. Create borrow token instruction
    // Borrow token is zero-size type, runtime equivalent to Mov.
    // This instruction's presence allows borrow checker to perform
    // flow-sensitive analysis.
    instructions.push(Instruction::Borrow {
        dst: Operand::Local(result_reg),
        src: Operand::Local(inner_reg),
        mutable: *mutable,
    });
}
```

**Key Points**:
- Borrow token is ZST, no actual creation needed at runtime
- `Borrow` instruction degenerates to `Mov` at runtime (dst = src)
- But borrow checker detects conflicts at compile-time by analyzing `Borrow`/`Release` instructions

---

## P0-3: execute.rs — Interpreter Borrow Handling

**Problem**: Interpreter has no borrow-related handling, `RuntimeValue` has no borrow variant.

**File**: `src/backends/interpreter/executor/execute.rs`

**Design Plan**: Borrow tokens are ZST, no special handling needed at runtime. `Borrow` instruction is equivalent to `Mov`, `Release` instruction is equivalent to `Nop`.

**Added match Branch**:

```rust
BytecodeInstr::Borrow { dst, src, mutable: _ } => {
    // Borrow token is zero-size type, runtime equivalent to Mov
    let val = frame
        .registers
        .get(src.0 as usize)
        .cloned()
        .unwrap_or(RuntimeValue::Unit);
    frame.set_register(dst.0 as usize, val);
    frame.advance();
}
BytecodeInstr::Release { src: _ } => {
    // Borrow token release, no operation at runtime
    frame.advance();
}
```

**Key Design Decision**: No need to add `Borrow` variant to `RuntimeValue`. All semantics of borrow tokens are guaranteed at compile-time by the borrow checker; at runtime, only value passing occurs.

---

## P1-3: bytecode.rs — Borrow/Release Bytecode Instructions

**Problem**: `BytecodeInstr` enum has no Borrow/Release, IR → bytecode conversion cannot handle these instructions.

**File**: `src/middle/core/bytecode.rs`

**Locations Requiring Modification**:

### 1. `BytecodeInstr` Enum (around L322, after Arc operations)

```rust
// =====================
// Borrow Token Operations
// =====================
/// Create borrow token (ZST, runtime ≈ Mov)
Borrow {
    dst: Reg,
    src: Reg,
    mutable: bool,
},
/// Release borrow token (ZST, runtime ≈ Nop)
Release {
    src: Reg,
},
```

### 2. `Opcode` Enum

```rust
Borrow,
Release,
```

### 3. `opcode()` Method

```rust
BytecodeInstr::Borrow { .. } => Opcode::Borrow,
BytecodeInstr::Release { .. } => Opcode::Release,
```

### 4. `size()` Method

```rust
BytecodeInstr::Borrow { .. } => 5,   // dst(2) + src(2) + mutable(1)
BytecodeInstr::Release { .. } => 2,  // src(2)
```

### 5. IR → Bytecode Conversion (`ir_to_bytecode` or equivalent function)

When handling `Instruction::Borrow`:
```rust
Instruction::Borrow { dst, src, mutable } => {
    BytecodeInstr::Borrow {
        dst: self.operand_to_reg(dst),
        src: self.operand_to_reg(src),
        mutable: *mutable,
    }
}
Instruction::Release(src) => {
    BytecodeInstr::Release {
        src: self.operand_to_reg(src),
    }
}
```

### 6. Bytecode Deserialization (`decode_instructions`)

```rust
Opcode::Borrow => {
    // Borrow: dst(2) + src(2) + mutable(1)
    if instr.operands.len() >= 5 {
        let dst = u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
        let src = u16::from_le_bytes([instr.operands[2], instr.operands[3]]);
        let mutable = instr.operands[4] != 0;
        decoded_instructions.push(BytecodeInstr::Borrow {
            dst: Reg(dst),
            src: Reg(src),
            mutable,
        });
    }
}
Opcode::Release => {
    // Release: src(2)
    if instr.operands.len() >= 2 {
        let src = u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
        decoded_instructions.push(BytecodeInstr::Release { src: Reg(src) });
    }
}
```

---

## P2-3: bytecode.rs — Real `From<MonoType>` Implementation

**Problem**: `From<MonoType> for IrType` is a placeholder stub, all types map to `IrType::Void`.

**File**: `src/middle/core/bytecode.rs` L1418-1424

**Current Code**:
```rust
impl From<MonoType> for IrType {
    fn from(_: MonoType) -> Self {
        IrType::Void  // placeholder
    }
}
```

**Fix**:
```rust
impl From<MonoType> for IrType {
    fn from(ty: MonoType) -> Self {
        match ty {
            MonoType::Int(_) => IrType::I64,
            MonoType::Float(_) => IrType::F64,
            MonoType::Bool => IrType::Bool,
            MonoType::Char => IrType::Char,
            MonoType::String => IrType::String,
            MonoType::Bytes => IrType::Bytes,
            MonoType::Void => IrType::Void,
            MonoType::List(_) => IrType::List,
            MonoType::Tuple(_) => IrType::Tuple,
            MonoType::Struct(_) => IrType::Struct,
            MonoType::Enum(_) => IrType::Enum,
            MonoType::Fn { .. } => IrType::Function,
            MonoType::Arc(_) => IrType::Arc,
            MonoType::Weak(_) => IrType::Weak,
            MonoType::Ref { .. } => IrType::Void,  // ZST, no runtime representation
            _ => IrType::Void,
        }
    }
}
```

**Note**: Verify that `IrType` enum has these variants defined. If not, need to extend `IrType` first.

---

## P2-1: ir_gen.rs — MakeClosure ZST Optimization

**Problem**: When `&T` tokens are captured, meaningless env overhead is generated.

**File**: `src/middle/core/ir_gen.rs` L3196-3198

**Current Code**:
```rust
for captured in &captured_vars {
    if let Some(local_idx) = self.lookup_local(&captured.name) {
        // TODO: ZST optimization — if the captured variable is ZST (e.g., &T token),
        // it should be skipped since it has no runtime representation.
        env_vars.push(Operand::Local(local_idx));
    }
}
```

**Fix**:
```rust
for captured in &captured_vars {
    if let Some(local_idx) = self.lookup_local(&captured.name) {
        // ZST optimization: borrow token is zero-size type, skip env
        if let Some(type_result) = &self.type_result {
            if let Some(mono_type) = type_result.local_var_types.get(&captured.name) {
                if matches!(mono_type, MonoType::Ref { .. }) {
                    continue;  // ZST, no need to add to env
                }
            }
        }
        env_vars.push(Operand::Local(local_idx));
    }
}
```

---

## P1-2: Brand Mechanism (Can Be Deferred)

**Problem**: Only uses variable name strings for source tracking, no compile-time unique ID.

**Impact**: Current variable name tracking is sufficient to support basic functionality. Brand mechanism is a defensive enhancement to prevent:
1. Variables from different sources with the same name being incorrectly identified as from the same source
2. Tokens being "forged" (internal compiler consistency)

**Deferral Reason**:
- Currently all variables within the same function are unique (SSA or scope isolation)
- Brand mechanism is an internal compiler optimization, does not affect user-visible behavior
- Implementation complexity is high, requires adding `brand_id: u64` and derivation chain to `BorrowToken`

**Recommendation**: Implement before entering LLVM backend, after P0/P1 completion.

---

## Test Strategy

### New Tests

1. **solver.rs**: `&T` Dup / `&mut T` non-Dup unit tests
2. **ir_gen.rs**: `Expr::Borrow` IR generation test (verify `Instruction::Borrow` generation)
3. **execute.rs**: Borrow token runtime test (verify `Borrow` ≈ `Mov`)
4. **borrow_checker.rs**: Existing 11 tests, verify `Borrow`/`Release` instruction integration

### Regression Tests

- `cargo test` full pass (currently 2125 tests)
- `cargo build` compiles without warnings

---

## File Modification Checklist

| File | Change Type | Priority |
|------|-------------|----------|
| `src/frontend/core/typecheck/traits/solver.rs` | +1 line match branch | P1-1 |
| `src/frontend/core/typecheck/traits/send_sync.rs` | Delete file | P2-2 |
| `src/frontend/core/typecheck/traits/mod.rs` | -1 line mod declaration | P2-2 |
| `src/frontend/core/typecheck/traits/auto_derive.rs` | Remove Send/Sync entries | P2-2 |
| `src/middle/core/ir.rs` | +10 lines (Borrow/Release instructions) | P0-1 |
| `src/middle/core/ir_gen.rs` | +15 lines (Expr::Borrow branch + ZST optimization) | P0-2, P2-1 |
| `src/middle/core/bytecode.rs` | +30 lines (instructions + Opcode + encode/decode) | P1-3, P2-3 |
| `src/backends/interpreter/executor/execute.rs` | +12 lines (Borrow/Release handling) | P0-3 |

**Total Changes**: Approximately 70 lines of new code + 1 file deletion.